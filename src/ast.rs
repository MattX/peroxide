// Copyright 2018-2020 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Turns a List representing a toplevel element in a Scheme program into an AST.
//!
//! This step is also responsible for computing all references. This is not great for separation
//! of concerns, but we need to keep track of the environment at the AST stage anyways to handle
//! macros and redefined keywords. Computing references here simplifies the compiler while not
//! making the AST parser much more complex.
//!
//! ### Future work / notes:
//!
//! Once the data has been read, we can drop all of the code we've read and keep only the quotes.
//! I think the easiest way to do this would be to use two separate arenas for the pre-AST and
//! post-AST values.
//!
//! Another small thing is dealing with loopy trees as allowed by R7RS.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use arena::Arena;
use environment::{
    get_toplevel_afi, ActivationFrame, Environment, EnvironmentValue, Macro, RcAfi, RcEnv,
};
use heap::{PoolPtr, RootPtr};
use primitives::SyntacticClosure;
use util::check_len;
use value::{list_from_vec, pretty_print, vec_from_list, Value};
use VmState;
use {compile, vm};
use {compile_run, environment};

const MAX_MACRO_EXPANSION: usize = 1000;

#[derive(Debug)]
pub enum SyntaxElement {
    Reference(Box<Reference>),
    Quote(Box<Quote>),
    If(Box<If>),
    Begin(Box<Begin>),
    Lambda(Box<Lambda>),
    Set(Box<Set>),
    Application(Box<Application>),
}

#[derive(Debug)]
pub struct Reference {
    pub altitude: usize,
    pub depth: usize,
    pub index: usize,
}

#[derive(Debug)]
pub struct Quote {
    pub quoted: RootPtr,
}

#[derive(Debug)]
pub struct If {
    pub cond: SyntaxElement,
    pub t: SyntaxElement,
    pub f: Option<SyntaxElement>,
}

#[derive(Debug)]
pub struct Begin {
    pub expressions: Vec<SyntaxElement>,
}

// The activation frame in a lambda has the formals, then all inner defines. In other words there
// are (num formals) + (num defines) variables in the topmost frame.
pub struct Lambda {
    pub env: RcEnv,
    pub arity: usize,
    pub dotted: bool,
    pub defines: Vec<SyntaxElement>,
    pub expressions: Vec<SyntaxElement>,
    pub name: Option<String>,
}

impl fmt::Debug for Lambda {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Lambda{{ name = {:?} arity = {}, dotted = {}, defines = {:?}, expressions = {:?} }}",
            self.name, self.arity, self.dotted, self.defines, self.expressions
        )
    }
}

#[derive(Debug)]
pub struct Set {
    pub altitude: usize,
    pub depth: usize,
    pub index: usize,
    pub value: SyntaxElement,
}

#[derive(Debug)]
pub struct Application {
    pub function: SyntaxElement,
    pub args: Vec<SyntaxElement>,
}

#[derive(Debug)]
pub enum Identifier {
    Bare(String),
    SyntacticClosure(PoolPtr),
}

/// Structure that holds a function's formal argument list.
/// `(x y z)` will be represented as `Formals { values: [x, y, z], rest: None }`
/// `(x y . z)` will be represented as `Formals { values: [x, y], rest: Some(z) }`
#[derive(Debug)]
struct Formals {
    pub values: Vec<DefineTarget>,
    pub rest: Option<DefineTarget>,
}

/// Parses an expression into an AST (aka `SyntaxElement`)
///
/// Annoyingly enough, we need a full `VmState` passed everywhere here, because macros
/// need to be executed and can add new code.
///
/// **parse() requires `value` to be rooted**, but it won't tell you that because it's sneaky.
/// `value` isn't a `RootPtr` just because it would be expensive to root and unroot values
/// constantly during parsing. Since the top-level value representing the expression to parse is
/// rooted, we can get away with not rooting stuff as we go down the expression.
pub fn parse(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    value: PoolPtr,
) -> Result<SyntaxElement, String> {
    let _value_hold = arena.root(value);
    let (env, value) = resolve_syntactic_closure(arena, env, value)?;
    match arena.get(value) {
        Value::Symbol(s) => Ok(SyntaxElement::Reference(Box::new(construct_reference(
            &env, af_info, s,
        )?))),
        Value::EmptyList => Err("Cannot evaluate empty list".into()),
        Value::Pair(car, cdr) => {
            let car = car.get();
            let cdr = cdr.get();
            parse_pair(arena, vms, &env, af_info, car, cdr)
        }
        _ => Ok(SyntaxElement::Quote(Box::new(Quote {
            quoted: arena.root(value),
        }))),
    }
}

fn construct_reference(env: &RcEnv, afi: &RcAfi, name: &str) -> Result<Reference, String> {
    let mut env = env.borrow_mut();
    match env.get(name) {
        Some(EnvironmentValue::Variable(v)) => Ok(Reference {
            altitude: v.altitude,
            depth: afi.borrow().altitude - v.altitude,
            index: v.index,
        }),
        Some(_) => Err(format!(
            "Illegal reference to {}, which is not a variable.",
            name
        )),
        None => {
            // TODO: remove this, or find a better way to surface it.
            println!(
                "Warning: reference to undefined variable {} in {:?}.",
                name, env
            );
            let index = env.define_toplevel(name, afi);
            Ok(Reference {
                altitude: 0,
                depth: afi.borrow().altitude,
                index,
            })
        }
    }
}

fn parse_pair(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    car: PoolPtr,
    cdr: PoolPtr,
) -> Result<SyntaxElement, String> {
    let rest = vec_from_list(arena, cdr)?;
    let (car_env, resolved_car) = resolve_syntactic_closure(arena, env, car)?;
    match arena.get(resolved_car) {
        Value::Symbol(s) => match match_symbol(&car_env, s) {
            Symbol::Quote => parse_quote(arena, &env, &rest, false),
            Symbol::SyntaxQuote => parse_quote(arena, &env, &rest, true),
            Symbol::If => parse_if(arena, vms, &env, af_info, &rest),
            Symbol::Begin => parse_begin(arena, vms, &env, af_info, &rest),
            Symbol::Lambda => parse_lambda(arena, vms, &env, af_info, &rest),
            Symbol::Set => parse_set(arena, vms, &env, af_info, &rest),
            Symbol::Define => parse_define(arena, vms, &env, af_info, &rest),
            Symbol::DefineSyntax => parse_define_syntax(arena, vms, &env, af_info, &rest),
            Symbol::LetSyntax => parse_let_syntax(arena, vms, &env, af_info, &rest, false),
            Symbol::LetrecSyntax => parse_let_syntax(arena, vms, &env, af_info, &rest, true),
            Symbol::Macro(m) => {
                // TODO fix this to avoid reconstructing the pair
                let expr = arena.insert(Value::Pair(Cell::new(car), Cell::new(cdr)));
                let expanded = expand_macro_full(arena, vms, &env, m, expr)?;
                parse(arena, vms, &env, af_info, expanded)
            }
            _ => parse_application(arena, vms, &env, af_info, car, &rest),
        },
        _ => parse_application(arena, vms, &env, af_info, car, &rest),
    }
}

fn parse_quote(
    arena: &Arena,
    env: &RcEnv,
    rest: &[PoolPtr],
    syntax: bool,
) -> Result<SyntaxElement, String> {
    if rest.len() != 1 {
        Err(format!("quote expected 1 argument, got {}.", rest.len()))
    } else if syntax {
        Ok(SyntaxElement::Quote(Box::new(Quote {
            quoted: arena.root(rest[0]),
        })))
    } else {
        let quoted = arena.root(strip_syntactic_closure(arena, env, rest[0]));
        Ok(SyntaxElement::Quote(Box::new(Quote { quoted })))
    }
}

fn parse_if(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(3))?;
    let cond = parse(arena, vms, env, af_info, rest[0])?;
    let t = parse(arena, vms, env, af_info, rest[1])?;
    let f_s: Option<Result<_, _>> = rest.get(2).map(|e| parse(arena, vms, env, af_info, *e));

    // This dark magic swaps the option and the result (then `?`s the result)
    // https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/option_result.html
    let f: Option<_> = f_s.map_or(Ok(None), |r| r.map(Some))?;
    Ok(SyntaxElement::If(Box::new(If { cond, t, f })))
}

fn parse_begin(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(1), None)?;
    let expressions = rest
        .iter()
        .map(|e| parse(arena, vms, env, af_info, *e))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Begin(Box::new(Begin { expressions })))
}

fn parse_lambda(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), None)?;
    parse_split_lambda(
        arena,
        vms,
        env,
        af_info,
        rest[0],
        &rest[1..rest.len()],
        None,
    )
}

fn parse_split_lambda(
    arena: &Arena,
    vms: &mut VmState,
    outer_env: &RcEnv,
    af_info: &RcAfi,
    formals: PoolPtr,
    body: &[PoolPtr],
    name: Option<String>,
) -> Result<SyntaxElement, String> {
    let formals = parse_formals(arena, formals)?;
    let inner_afi = environment::extend_af_info(af_info);
    let raw_env = Environment::new(Some(outer_env.clone()));
    let inner_env = Rc::new(RefCell::new(raw_env));
    let mut targets = Vec::new();

    // TODO check that the formals are all distinct.
    for define_target in formals.values.iter() {
        define_in_env(arena, &inner_env, &inner_afi, define_target, true);
        targets.push(define_target.clone());
    }
    if let Some(define_target) = &formals.rest {
        define_in_env(arena, &inner_env, &inner_afi, define_target, true);
        targets.push(define_target.clone());
    }
    let (unparsed_defines, rest) = collect_internal_defines(arena, vms, &inner_env, body)?;
    for define_data in unparsed_defines.iter() {
        define_in_env(arena, &inner_env, &inner_afi, &define_data.target, false);
        targets.push(define_data.target.clone());
    }

    let defines = unparsed_defines
        .iter()
        .map(|define_data| {
            let value = define_data.value.parse(
                arena,
                vms,
                &inner_env,
                &inner_afi,
                define_data.target.get_name(arena),
            )?;
            if let Some(EnvironmentValue::Variable(v)) =
                get_in_env(arena, &inner_env, &define_data.target)
            {
                Ok(SyntaxElement::Set(Box::new(Set {
                    altitude: v.altitude,
                    depth: inner_afi.borrow().altitude - v.altitude,
                    index: v.index,
                    value,
                })))
            } else {
                panic!(
                    "Expected {} in {:?} to be a variable, was {:?}.",
                    define_data.target.show(arena),
                    inner_env,
                    get_in_env(arena, &inner_env, &define_data.target)
                );
            }
        })
        .collect::<Result<Vec<SyntaxElement>, String>>()?;

    let expressions = rest
        .iter()
        .map(|e| parse(arena, vms, &inner_env, &inner_afi, *e))
        .collect::<Result<Vec<_>, _>>()?;

    pop_envs(arena, &targets);
    if expressions.is_empty() {
        return Err("Lambda cannot have empty body".into());
    }
    Ok(SyntaxElement::Lambda(Box::new(Lambda {
        env: inner_env,
        arity: formals.values.len(),
        dotted: formals.rest.is_some(),
        defines,
        expressions,
        name,
    })))
}

fn parse_set(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(2))?;
    if let Some(dt) = get_define_target(arena, rest[0]) {
        let value = parse(arena, vms, env, af_info, rest[1])?;
        match get_in_env(arena, env, &dt) {
            Some(EnvironmentValue::Variable(v)) => Ok(SyntaxElement::Set(Box::new(Set {
                altitude: v.altitude,
                depth: af_info.borrow().altitude - v.altitude,
                index: v.index,
                value,
            }))),
            Some(_) => Err(format!(
                "Trying to set non-variable `{}`",
                dt.get_name(arena)
            )),
            None => Err(format!(
                "Trying to set undefined value `{}`",
                dt.get_name(arena)
            )),
        }
    } else {
        Err(format!(
            "Expected symbol as target of set!, got `{}`",
            pretty_print(arena, rest[0])
        ))
    }
}

/// Parses toplevel defines. Inner defines have different semantics and are parsed differently
/// (see [collect_internal_defines]).
fn parse_define(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    // TODO the actual check should not be on activation frame altitude, but on syntactic
    //      toplevelness. (eg `(define x (define y 1))` should not work).
    if af_info.borrow().altitude != 0 {
        return Err(format!(
            "Define in illegal position: {}",
            pretty_print(arena, list_from_vec(arena, rest))
        ));
    }
    let define_data = get_define_data(arena, rest)?;

    // TODO: don't do this and instead allow defining syncloses at top level?
    let symbol = define_data.target.coerce_symbol();
    let index = env.borrow_mut().define_if_absent(&symbol, af_info, false);
    let value = define_data.value.parse(arena, vms, env, af_info, symbol)?;
    Ok(SyntaxElement::Set(Box::new(Set {
        altitude: 0,
        depth: af_info.borrow().altitude,
        index,
        value,
    })))
}

#[derive(Debug, Clone)]
enum DefineTarget {
    Bare(String),
    SyntacticClosure(PoolPtr),
}

impl DefineTarget {
    fn coerce_symbol(&self) -> String {
        match self {
            DefineTarget::Bare(s) => s.clone(),
            _ => panic!("Coercing syntactic closure into symbol."),
        }
    }

    fn get_name(&self, arena: &Arena) -> String {
        match self {
            DefineTarget::Bare(s) => s.clone(),
            DefineTarget::SyntacticClosure(v) => {
                let sc = arena.try_get_syntactic_closure(*v).unwrap();
                let symbol = arena.try_get_symbol(sc.expr).unwrap();
                symbol.into()
            }
        }
    }

    fn show(&self, arena: &Arena) -> String {
        match self {
            DefineTarget::Bare(s) => format!("Bare({})", s),
            DefineTarget::SyntacticClosure(v) => format!("Sc({})", pretty_print(arena, *v)),
        }
    }
}

#[derive(Debug)]
enum DefineValue {
    Value(PoolPtr),
    Lambda {
        formals: PoolPtr,
        body: Vec<PoolPtr>,
    },
}

impl DefineValue {
    pub fn parse(
        &self,
        arena: &Arena,
        vms: &mut VmState,
        env: &RcEnv,
        af_info: &RcAfi,
        name: String,
    ) -> Result<SyntaxElement, String> {
        match self {
            DefineValue::Value(v) => parse(arena, vms, env, af_info, *v),
            DefineValue::Lambda { formals, body } => {
                parse_split_lambda(arena, vms, env, af_info, *formals, &body, Some(name))
            }
        }
    }
}

#[derive(Debug)]
struct DefineData {
    pub target: DefineTarget,
    pub value: DefineValue,
}

fn get_define_data(arena: &Arena, rest: &[PoolPtr]) -> Result<DefineData, String> {
    let res = if let Some(target) = get_define_target(arena, rest[0]) {
        check_len(rest, Some(2), Some(2))?;
        DefineData {
            target,
            value: DefineValue::Value(rest[1]),
        }
    } else {
        get_lambda_define_value(arena, rest)?
    };
    Ok(res)
}

/// Helper method to parse direct lambda defines `(define (x y z) y z)`.
fn get_lambda_define_value(arena: &Arena, rest: &[PoolPtr]) -> Result<DefineData, String> {
    check_len(rest, Some(2), None)?;
    if let Value::Pair(car, cdr) = arena.get(rest[0]) {
        if let Value::Symbol(s) = arena.get(car.get()) {
            let variable = s.clone();
            Ok(DefineData {
                target: DefineTarget::Bare(variable),
                value: DefineValue::Lambda {
                    formals: cdr.get(),
                    body: rest[1..rest.len()].to_vec(),
                },
            })
        } else {
            Err(format!(
                "Expected symbol for method name in define method, got `{}`.",
                pretty_print(arena, car.get())
            ))
        }
    } else {
        Err(format!(
            "Expected symbol or formals as target of define, got `{}`.",
            pretty_print(arena, rest[0])
        ))
    }
}

fn parse_application(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    fun: PoolPtr,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    let function = parse(arena, vms, env, af_info, fun)?;
    let args = rest
        .iter()
        .map(|e| parse(arena, vms, env, af_info, *e))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Application(Box::new(Application {
        function,
        args,
    })))
}

fn parse_formals(arena: &Arena, formals: PoolPtr) -> Result<Formals, String> {
    let mut values = Vec::new();
    let mut formal = formals;
    loop {
        if let Some(dt) = get_define_target(arena, formal) {
            return Ok(Formals {
                values,
                rest: Some(dt),
            });
        } else {
            match arena.get(formal) {
                Value::EmptyList => return Ok(Formals { values, rest: None }),
                Value::Pair(car, cdr) => {
                    if let Some(dt) = get_define_target(arena, car.get()) {
                        values.push(dt);
                        formal = cdr.get();
                    } else {
                        return Err(format!(
                            "Malformed formals: {}.",
                            arena.get(formals).pretty_print(arena)
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Malformed formals: {}.",
                        arena.get(formals).pretty_print(arena)
                    ));
                }
            }
        }
    }
}

fn parse_define_syntax(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
) -> Result<SyntaxElement, String> {
    // TODO the actual check should not be on activation frame altitude, but on syntactic
    //      toplevelness. (eg `(define x (define y 1))` should not work).
    if af_info.borrow().altitude != 0 {
        return Err("Illegally placed define-syntax.".into());
    }
    check_len(rest, Some(2), Some(2))?;

    let symbol = arena
        .try_get_symbol(rest[0])
        .ok_or_else(|| {
            format!(
                "define-syntax: target must be symbol, not {}.",
                pretty_print(arena, rest[0])
            )
        })?
        .to_string();
    let mac = make_macro(arena, env, af_info, vms, rest[1])?;
    env.borrow_mut().define_macro(&symbol, mac, env.clone());

    // TODO remove this somehow
    Ok(SyntaxElement::Quote(Box::new(Quote {
        quoted: arena.root(arena.unspecific),
    })))
}

fn parse_let_syntax(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    af_info: &RcAfi,
    rest: &[PoolPtr],
    rec: bool,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), None)?;
    let bindings = vec_from_list(arena, rest[0])?;
    let inner_env = Rc::new(RefCell::new(Environment::new(Some(env.clone()))));
    let definition_env = if rec { env } else { &inner_env };
    for b in bindings.iter() {
        let binding = vec_from_list(arena, *b)?;
        check_len(&binding, Some(2), Some(2))?;

        let symbol = arena
            .try_get_symbol(binding[0])
            .ok_or_else(|| {
                format!(
                    "let-syntax: target must be symbol, not {}.",
                    pretty_print(arena, rest[0])
                )
            })?
            .to_string();
        let mac = make_macro(arena, env, af_info, vms, binding[1])?;
        inner_env
            .borrow_mut()
            .define_macro(&symbol, mac, definition_env.clone());
    }

    // Letrec and letrec syntax are allowed to have internal defines for some reason. We just
    // create a lambda with no args and the body, and apply it immediately with no args.
    let lambda = parse_split_lambda(
        arena,
        vms,
        &inner_env,
        af_info,
        arena.empty_list,
        &rest[1..],
        Some("[let-syntax inner lambda]".into()),
    )?;
    Ok(SyntaxElement::Application(Box::new(Application {
        function: lambda,
        args: vec![],
    })))
}

fn make_macro(
    arena: &Arena,
    env: &RcEnv,
    af_info: &RcAfi,
    vms: &mut VmState,
    val: PoolPtr,
) -> Result<RootPtr, String> {
    let mac = parse_compile_run_macro(arena, env, af_info, vms, val)?;
    let mac = arena.root(mac);
    match arena.get(mac.pp()) {
        Value::Lambda { .. } => Ok(mac), // TODO check the lambda takes 3 args
        _ => Err(format!(
            "macro must be a lambda, is {}",
            pretty_print(arena, mac.pp())
        )),
    }
}

/// Like parse_compile_run, but it creates a fake environment to evaluate the macro in.
// TODO: refactor common code with parse_compile_run
fn parse_compile_run_macro(
    arena: &Arena,
    env: &RcEnv,
    af_info: &RcAfi,
    vms: &mut VmState,
    val: PoolPtr,
) -> Result<PoolPtr, String> {
    let syntax_tree =
        parse(arena, vms, env, af_info, val).map_err(|e| format!("syntax error: {}", e))?;
    arena
        .get_activation_frame(vms.global_frame.pp())
        .borrow_mut()
        .ensure_index(arena, get_toplevel_afi(af_info).borrow().entries);

    let frame = make_frame(arena, vms.global_frame.pp(), af_info);

    let code = compile::compile_toplevel(arena, &syntax_tree, vms.global_environment.clone());
    // println!(" => {:?}", &state.code[start_pc..state.code.len()]);
    let code = arena.root(code);
    vm::run(arena, code, 0, vms.global_frame.pp(), frame)
        .map(|v| v.pp())
        .map_err(|e| format!("runtime error: {}", pretty_print(arena, e.pp())))
}

fn make_frame(arena: &Arena, global_frame: PoolPtr, af_info: &RcAfi) -> PoolPtr {
    let parent = if let Some(p) = af_info.borrow().parent.clone() {
        p
    } else {
        return global_frame;
    };
    let entries = af_info.borrow().entries;
    let mut frame = ActivationFrame {
        parent: Some(make_frame(arena, global_frame, &parent)),
        values: Vec::with_capacity(entries),
    };
    frame.values.resize(entries, arena.undefined);
    arena.insert(Value::ActivationFrame(RefCell::new(frame)))
}

fn expand_macro_full(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    mac: Macro,
    expr: PoolPtr,
) -> Result<PoolPtr, String> {
    let expr = arena.root(expr);
    let mut expanded = expand_macro(arena, vms, env, mac, expr)?;
    let mut macro_count = 0;
    while let Some(m) = get_macro(arena, env, expanded.pp()) {
        macro_count += 1;
        if macro_count > MAX_MACRO_EXPANSION {
            return Err("Maximum macro expansion depth reached.".into());
        }
        expanded = expand_macro(arena, vms, env, m, expanded)?;
    }
    Ok(expanded.pp())
}

fn expand_macro(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    mac: Macro,
    expr: RootPtr,
) -> Result<RootPtr, String> {
    let definition_environment = Value::Environment(mac.definition_environment.clone());
    let usage_environment = Value::Environment(env.clone());
    arena.insert(Value::Integer(100.into()));
    let syntax_tree = SyntaxElement::Application(Box::new(Application {
        function: SyntaxElement::Quote(Box::new(Quote { quoted: mac.lambda })),
        args: vec![
            SyntaxElement::Quote(Box::new(Quote { quoted: expr })),
            SyntaxElement::Quote(Box::new(Quote {
                quoted: arena.insert_rooted(usage_environment),
            })),
            SyntaxElement::Quote(Box::new(Quote {
                quoted: arena.insert_rooted(definition_environment),
            })),
        ],
    }));
    compile_run(arena, vms, &syntax_tree)
}

fn get_macro(arena: &Arena, env: &RcEnv, expr: PoolPtr) -> Option<Macro> {
    match arena.get(expr) {
        Value::Pair(car, _cdr) => {
            let (res_env, res_car) = resolve_syntactic_closure(arena, env, car.get()).unwrap();
            match arena.get(res_car) {
                Value::Symbol(s) => match match_symbol(&res_env, &s) {
                    Symbol::Macro(m) => Some(m),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

enum Symbol {
    Quote,
    SyntaxQuote,
    If,
    Begin,
    Lambda,
    Set,
    Define,
    DefineSyntax,
    LetSyntax,
    LetrecSyntax,
    Macro(Macro),
    Variable,
}

fn match_symbol(env: &RcEnv, sym: &str) -> Symbol {
    match env.borrow().get(sym) {
        None => match sym {
            "quote" => Symbol::Quote,
            "syntax-quote" => Symbol::SyntaxQuote,
            "if" => Symbol::If,
            "begin" => Symbol::Begin,
            "lambda" => Symbol::Lambda,
            "set!" => Symbol::Set,
            "define" => Symbol::Define,
            "define-syntax" => Symbol::DefineSyntax,
            "let-syntax" => Symbol::LetSyntax,
            "letrec-syntax" => Symbol::LetrecSyntax,
            _ => Symbol::Variable,
        },
        Some(EnvironmentValue::Macro(m)) => Symbol::Macro(m),
        Some(EnvironmentValue::Variable(_)) => Symbol::Variable,
    }
}

#[allow(clippy::type_complexity)]
fn collect_internal_defines(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    body: &[PoolPtr],
) -> Result<(Vec<DefineData>, Vec<PoolPtr>), String> {
    // TODO figure out a nice way to push macro expanded, non-define values. Right know
    //      we'll perform macro expansion both here and at the actual parse site.
    // TODO support internal macro definitions

    let mut defines = Vec::new();
    let mut rest = Vec::new();
    let mut i = 0 as usize;

    for statement in body.iter() {
        let expanded_statement = if let Some(m) = get_macro(arena, env, *statement) {
            expand_macro_full(arena, vms, env, m, *statement)?
        } else {
            *statement
        };
        if let Value::Pair(car, cdr) = arena.get(expanded_statement) {
            let (res_env, res_car) = resolve_syntactic_closure(arena, env, car.get())?;
            if let Value::Symbol(s) = arena.get(res_car) {
                match match_symbol(&res_env, s) {
                    Symbol::Define => {
                        let rest = vec_from_list(arena, cdr.get())?;
                        let dv = get_define_data(arena, &rest)?;
                        defines.push(dv);
                    }
                    Symbol::Begin => {
                        let expressions = vec_from_list(arena, cdr.get())?;
                        let (d, rest) = collect_internal_defines(arena, vms, env, &expressions)?;
                        if !rest.is_empty() {
                            return Err(
                                "Inner begin in define section may only contain definitions."
                                    .into(),
                            );
                        }
                        defines.extend(d.into_iter());
                    }
                    Symbol::Macro(_) => panic!("Macro in fully expanded statement."),
                    _ => break,
                }
            } else {
                break;
            }
        } else {
            break;
        }
        i += 1;
    }

    rest.extend(&body[i..]);
    assert_eq!(body.len(), defines.len() + rest.len());
    Ok((defines, rest))
}

fn resolve_syntactic_closure(
    arena: &Arena,
    env: &RcEnv,
    value: PoolPtr,
) -> Result<(RcEnv, PoolPtr), String> {
    if let Value::SyntacticClosure(SyntacticClosure {
        closed_env,
        free_variables,
        expr,
    }) = arena.get(value)
    {
        let closed_env = arena
            .try_get_environment(*closed_env.borrow())
            .expect("Syntactic closure created with non-environment argument.");
        let inner_env = environment::filter(closed_env, env, free_variables)?;
        resolve_syntactic_closure(arena, &inner_env, *expr)
    } else {
        Ok((env.clone(), value))
    }
}

fn strip_syntactic_closure(arena: &Arena, env: &RcEnv, value: PoolPtr) -> PoolPtr {
    if let Value::SyntacticClosure(SyntacticClosure { expr, .. }) = arena.get(value) {
        strip_syntactic_closure(arena, env, *expr)
    } else {
        value
    }
}

fn get_define_target(arena: &Arena, value: PoolPtr) -> Option<DefineTarget> {
    match arena.get(value) {
        Value::Symbol(s) => Some(DefineTarget::Bare(s.clone())),
        Value::SyntacticClosure(sc) => match arena.get(sc.expr) {
            Value::Symbol(_) => Some(DefineTarget::SyntacticClosure(value)),
            Value::SyntacticClosure(_) => get_define_target(arena, sc.expr),
            _ => None,
        },
        _ => None,
    }
}

fn define_in_env(
    arena: &Arena,
    env: &RcEnv,
    afi: &RcAfi,
    target: &DefineTarget,
    initialized: bool,
) {
    match target {
        DefineTarget::Bare(s) => {
            env.borrow_mut().define(s, afi, initialized);
        }
        DefineTarget::SyntacticClosure(val) => {
            let sc = arena.try_get_syntactic_closure(*val).unwrap();
            let name = arena.try_get_symbol(sc.expr).unwrap();
            let new_env = sc.push_env(arena);
            new_env.borrow_mut().define(name, afi, initialized);
        }
    }
}

fn get_in_env(arena: &Arena, env: &RcEnv, target: &DefineTarget) -> Option<EnvironmentValue> {
    match target {
        DefineTarget::Bare(s) => env.borrow().get(s),
        DefineTarget::SyntacticClosure(val) => {
            let sc = arena.try_get_syntactic_closure(*val).unwrap();
            let name = arena.try_get_symbol(sc.expr).unwrap();
            let closed_env = arena.try_get_environment(*sc.closed_env.borrow()).unwrap();
            closed_env.borrow().get(name)
        }
    }
}

fn pop_envs(arena: &Arena, targets: &[DefineTarget]) {
    for target in targets {
        if let DefineTarget::SyntacticClosure(val) = target {
            let sc = arena.try_get_syntactic_closure(*val).unwrap();
            sc.pop_env(arena);
        }
    }
}
