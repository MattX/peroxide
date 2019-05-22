// Copyright 2018-2019 Matthieu Felix
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

use arena::Arena;
use environment::{Environment, EnvironmentValue, Macro, RcEnv};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use util::{check_len, max_optional};
use value::{pretty_print, vec_from_list, Value};
use {compile_run, environment};
use {parse_compile_run, VmState};

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
    pub index: usize,
}

#[derive(Debug)]
pub struct Quote {
    pub quoted: usize,
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
}

impl fmt::Debug for Lambda {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Lambda {{ arity = {}, dotted = {}, defines = {:?}, expressions = {:?} }}",
            self.arity, self.dotted, self.defines, self.expressions
        )
    }
}

#[derive(Debug)]
pub struct Set {
    pub altitude: usize,
    pub index: usize,
    pub value: SyntaxElement,
}

#[derive(Debug)]
pub struct Application {
    pub function: SyntaxElement,
    pub args: Vec<SyntaxElement>,
}

/// Structure that holds a function's formal argument list.
/// `(x y z)` will be represented as `Formals { values: [x, y, z], rest: None }`
/// `(x y . z)` will be represented as `Formals { values: [x, y], rest: Some(z) }`
#[derive(Debug)]
pub struct Formals {
    pub values: Vec<String>,
    pub rest: Option<String>,
}

/// Parses an expression into an AST (aka `SyntaxElement`)
///
/// Annoyingly enough, we need a full `VmState` passed everywhere here, because macros
/// need to be executed and can add new code.
pub fn parse(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    value: usize,
    altitude: usize,
) -> Result<SyntaxElement, String> {
    let (env, value) = resolve_syntactic_closure(arena, env, value)?;
    match arena.get(value) {
        Value::Symbol(s) => Ok(SyntaxElement::Reference(Box::new(construct_reference(
            &env, s,
        )?))),
        Value::EmptyList => Err("Cannot evaluate empty list".into()),
        Value::Pair(car, cdr) => {
            parse_pair(arena, vms, &env, *car.borrow(), *cdr.borrow(), altitude)
        }
        _ => Ok(SyntaxElement::Quote(Box::new(Quote { quoted: value }))),
    }
}

fn construct_reference(env: &RcEnv, name: &str) -> Result<Reference, String> {
    let mut env = env.borrow_mut();
    match env.get(name) {
        Some(EnvironmentValue::Variable(v)) => Ok(Reference {
            altitude: v.altitude,
            index: v.index,
        }),
        Some(_) => Err(format!(
            "Illegal reference to {}, which is not a variable.",
            name
        )),
        None => {
            let index = env.define_implicit(name);
            // TODO: remove this, or find a better way to surface it.
            println!("Warning: reference to undefined variable {}", name);
            Ok(Reference { altitude: 0, index })
        }
    }
}

fn parse_pair(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    car: usize,
    cdr: usize,
    altitude: usize,
) -> Result<SyntaxElement, String> {
    let rest = vec_from_list(arena, cdr)?;
    let (env, car) = resolve_syntactic_closure(arena, env, car)?;
    match arena.get(car) {
        Value::Symbol(s) => match match_symbol(&env, s) {
            Symbol::Quote => parse_quote(&rest),
            Symbol::If => parse_if(arena, vms, &env, &rest, altitude),
            Symbol::Begin => parse_begin(arena, vms, &env, &rest, altitude),
            Symbol::Lambda => parse_lambda(arena, vms, &env, &rest, altitude),
            Symbol::Set => parse_set(arena, vms, &env, &rest, altitude),
            Symbol::Define => parse_define(arena, vms, &env, &rest, altitude),
            Symbol::DefineSyntax => parse_define_syntax(arena, vms, &env, &rest, altitude),
            Symbol::LetSyntax => parse_let_syntax(arena, vms, &env, &rest, false, altitude),
            Symbol::LetrecSyntax => parse_let_syntax(arena, vms, &env, &rest, true, altitude),
            Symbol::Macro(m) => {
                // TODO fix this to avoid reconstructing the pair
                let expr = arena.insert(Value::Pair(RefCell::new(car), RefCell::new(cdr)));
                let expanded = expand_macro_full(arena, vms, &env, m, expr)?;
                parse(arena, vms, &env, expanded, altitude)
            }
            _ => parse_application(arena, vms, &env, car, &rest, altitude),
        },
        _ => parse_application(arena, vms, &env, car, &rest, altitude),
    }
}

fn parse_quote(rest: &[usize]) -> Result<SyntaxElement, String> {
    if rest.len() != 1 {
        Err(format!("quote expected 1 argument, got {}.", rest.len()))
    } else {
        Ok(SyntaxElement::Quote(Box::new(Quote { quoted: rest[0] })))
    }
}

fn parse_if(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(3))?;
    let cond = parse(arena, vms, env, rest[0], altitude)?;
    let t = parse(arena, vms, env, rest[1], altitude)?;
    let f_s: Option<Result<_, _>> = rest.get(2).map(|e| parse(arena, vms, env, *e, altitude));

    // This dark magic swaps the option and the result (then `?`s the result)
    // https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/option_result.html
    let f: Option<_> = f_s.map_or(Ok(None), |r| r.map(Some))?;
    Ok(SyntaxElement::If(Box::new(If { cond, t, f })))
}

fn parse_begin(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(1), None)?;
    let expressions = rest
        .iter()
        .map(|e| parse(arena, vms, env, *e, altitude))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Begin(Box::new(Begin { expressions })))
}

fn parse_lambda(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), None)?;
    parse_split_lambda(arena, vms, env, rest[0], &rest[1..rest.len()], altitude)
}

// TODO parse internal defines
fn parse_split_lambda(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    formals: usize,
    body: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    let formals = parse_formals(arena, formals)?;
    let mut raw_env = Environment::new_initial(Some(env.clone()), &formals.values[..]);
    if let Some(s) = &formals.rest {
        raw_env.define(s, true);
    }
    let (unparsed_defines, rest) = collect_internal_defines(arena, vms, env, body)?;
    for (target, _expression) in unparsed_defines.iter() {
        raw_env.define(target, false);
    }
    let env = Rc::new(RefCell::new(raw_env));
    let defines = unparsed_defines
        .iter()
        .map(|(target, expression)| {
            let value = expression.parse(arena, vms, &env, altitude + 1)?;
            if let Some(EnvironmentValue::Variable(v)) = env.borrow().get(target) {
                Ok(SyntaxElement::Set(Box::new(Set {
                    altitude: v.altitude,
                    index: v.index,
                    value,
                })))
            } else {
                panic!("wat");
            }
        })
        .collect::<Result<Vec<SyntaxElement>, String>>()?;
    let expressions = rest
        .iter()
        .map(|e| parse(arena, vms, &env, *e, altitude + 1))
        .collect::<Result<Vec<_>, _>>()?;
    if expressions.is_empty() {
        return Err("Lambda cannot have empty body".into());
    }
    Ok(SyntaxElement::Lambda(Box::new(Lambda {
        env,
        arity: formals.values.len(),
        dotted: formals.rest.is_some(),
        defines,
        expressions,
    })))
}

fn parse_set(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(2))?;
    if let Value::Symbol(name) = arena.get(rest[0]) {
        let value = parse(arena, vms, env, rest[1], altitude)?;
        match env.borrow().get(name) {
            Some(EnvironmentValue::Variable(v)) => Ok(SyntaxElement::Set(Box::new(Set {
                altitude: v.altitude,
                index: v.index,
                value,
            }))),
            Some(_) => Err(format!("Trying to set non-variable `{}`", name)),
            None => Err(format!("Trying to set undefined value `{}`", name)),
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
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    if altitude != 0 {
        return Err("Define in illegal position.".into());
    }
    let (symbol, define_value) = get_define_value(arena, rest)?;
    let index = env.borrow_mut().define_if_absent(&symbol, false);
    let value = define_value.parse(arena, vms, env, altitude)?;
    Ok(SyntaxElement::Set(Box::new(Set {
        altitude: 0,
        index,
        value,
    })))
}

enum DefineValue {
    Value(usize),
    Lambda { formals: usize, body: Vec<usize> },
}

impl DefineValue {
    pub fn parse(
        &self,
        arena: &Arena,
        vms: &mut VmState,
        env: &RcEnv,
        altitude: usize,
    ) -> Result<SyntaxElement, String> {
        match self {
            DefineValue::Value(v) => parse(arena, vms, env, *v, altitude),
            DefineValue::Lambda { formals, body } => {
                parse_split_lambda(arena, vms, env, *formals, &body, altitude)
            }
        }
    }
}

fn get_define_value(arena: &Arena, rest: &[usize]) -> Result<(String, DefineValue), String> {
    let res = match arena.get(rest[0]) {
        Value::Symbol(s) => {
            check_len(rest, Some(2), Some(2))?;
            (s.clone(), DefineValue::Value(rest[1]))
        }
        _ => get_lambda_define_value(arena, rest)?,
    };
    Ok(res)
}

/// Helper method to parse direct lambda defines `(define (x y z) y z)`.
fn get_lambda_define_value(arena: &Arena, rest: &[usize]) -> Result<(String, DefineValue), String> {
    check_len(rest, Some(2), None)?;
    if let Value::Pair(car, cdr) = arena.get(rest[0]) {
        if let Value::Symbol(s) = arena.get(*car.borrow()) {
            let variable = s.clone();
            Ok((
                variable,
                DefineValue::Lambda {
                    formals: *cdr.borrow(),
                    body: rest[1..rest.len()].to_vec(),
                },
            ))
        } else {
            Err(format!(
                "Expected symbol for method name in define method, got `{}`.",
                pretty_print(arena, *car.borrow())
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
    fun: usize,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    let function = parse(arena, vms, env, fun, altitude)?;
    let args = rest
        .iter()
        .map(|e| parse(arena, vms, env, *e, altitude))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Application(Box::new(Application {
        function,
        args,
    })))
}

fn parse_formals(arena: &Arena, formals: usize) -> Result<Formals, String> {
    let mut values = Vec::new();
    let mut formal = formals;
    loop {
        match arena.get(strip_syntactic_closure(arena, formal)) {
            Value::Symbol(s) => {
                return Ok(Formals {
                    values,
                    rest: Some(s.clone()),
                })
            }
            Value::EmptyList => return Ok(Formals { values, rest: None }),
            Value::Pair(car, cdr) => {
                match arena.get(strip_syntactic_closure(arena, *car.borrow())) {
                    Value::Symbol(s) => {
                        values.push(s.clone());
                        formal = *cdr.borrow();
                    }
                    _ => {
                        return Err(format!(
                            "Malformed formals: {}.",
                            arena.get(formals).pretty_print(arena)
                        ))
                    }
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

fn parse_define_syntax(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    altitude: usize,
) -> Result<SyntaxElement, String> {
    if altitude != 0 {
        return Err("Illegally placed define-syntax.".into());
    }
    check_len(rest, Some(2), Some(2))?;

    let symbol = arena.try_get_symbol(rest[0]).ok_or_else(|| {
        format!(
            "define-syntax: target must be symbol, not {}.",
            pretty_print(arena, rest[0])
        )
    })?;
    let mac = make_macro(arena, vms, rest[1])?;
    env.borrow_mut().define_macro(symbol, mac, env.clone());

    // TODO remove this somehow
    Ok(SyntaxElement::Quote(Box::new(Quote {
        quoted: arena.unspecific,
    })))
}

fn parse_let_syntax(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    rest: &[usize],
    rec: bool,
    altitude: usize,
) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), None)?;
    let bindings = vec_from_list(arena, rest[0])?;
    let inner_env = Rc::new(RefCell::new(Environment::new_syntactic(env)));
    let definition_env = if rec { env } else { &inner_env };
    for b in bindings.iter() {
        let binding = vec_from_list(arena, *b)?;
        check_len(&binding, Some(2), Some(2))?;

        let symbol = arena.try_get_symbol(binding[0]).ok_or_else(|| {
            format!(
                "let-syntax: target must be symbol, not {}.",
                pretty_print(arena, rest[0])
            )
        })?;
        let mac = make_macro(arena, vms, binding[1])?;
        inner_env
            .borrow_mut()
            .define_macro(symbol, mac, definition_env.clone());
    }

    // Letrec and letrec syntax are allowed to have internal defines for some reason. We just
    // create a lambda with no args and the body, and apply it immediately with no args.
    let lambda = parse_split_lambda(
        arena,
        vms,
        &inner_env,
        arena.empty_list,
        &rest[1..],
        altitude,
    )?;
    Ok(SyntaxElement::Application(Box::new(Application {
        function: lambda,
        args: vec![],
    })))
}

fn make_macro(arena: &Arena, vms: &mut VmState, val: usize) -> Result<usize, String> {
    let mac = parse_compile_run(arena, vms, val)?;
    match arena.get(mac) {
        Value::Lambda { .. } => (), // TODO check the lambda takes 3 args
        _ => {
            return Err(format!(
                "Macro must be a Lambda, is {}",
                pretty_print(arena, mac)
            ))
        }
    };
    Ok(mac)
}

fn expand_macro_full(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    mac: Macro,
    expr: usize,
) -> Result<usize, String> {
    let mut expanded = expand_macro(arena, vms, env, mac, expr)?;
    while let Some(m) = get_macro(arena, env, expanded) {
        expanded = expand_macro(arena, vms, env, m, expanded)?;
    }
    println!("Expanded: {}", pretty_print(arena, expanded));
    Ok(expanded)
}

fn expand_macro(
    arena: &Arena,
    vms: &mut VmState,
    env: &RcEnv,
    mac: Macro,
    expr: usize,
) -> Result<usize, String> {
    let definition_environment = Value::Environment(mac.definition_environment.clone());
    let usage_environment = Value::Environment(env.clone());
    let syntax_tree = SyntaxElement::Application(Box::new(Application {
        function: SyntaxElement::Quote(Box::new(Quote { quoted: mac.lambda })),
        args: vec![
            SyntaxElement::Quote(Box::new(Quote { quoted: expr })),
            SyntaxElement::Quote(Box::new(Quote {
                quoted: arena.insert(usage_environment),
            })),
            SyntaxElement::Quote(Box::new(Quote {
                quoted: arena.insert(definition_environment),
            })),
        ],
    }));
    compile_run(arena, vms, &syntax_tree)
}

fn get_macro(arena: &Arena, env: &RcEnv, expr: usize) -> Option<Macro> {
    match arena.get(expr) {
        Value::Pair(car, _cdr) => match arena.get(*car.borrow()) {
            Value::Symbol(s) => match match_symbol(env, &s) {
                Symbol::Macro(m) => Some(m),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

enum Symbol {
    Quote,
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
    body: &[usize],
) -> Result<(Vec<(String, DefineValue)>, Vec<usize>), String> {
    // TODO figure out a nice way to push macro expanded, non-define values. Right know
    //      we'll perform macro expansion both here and at the actual parse site.

    let mut defines = Vec::new();
    let mut rest = Vec::new();
    let mut i = 0 as usize;

    for statement in body.iter() {
        let expanded_statement = if let Some(m) = get_macro(arena, env, *statement) {
            expand_macro_full(arena, vms, env, m, *statement)?
        } else {
            *statement
        };
        match arena.get(expanded_statement) {
            Value::Pair(car, cdr) => match arena.get(*car.borrow()) {
                Value::Symbol(s) => match match_symbol(env, s) {
                    Symbol::Define => {
                        let rest = vec_from_list(arena, *cdr.borrow())?;
                        let dv = get_define_value(arena, &rest)?;
                        defines.push(dv);
                    }
                    Symbol::Begin => {
                        let expressions = vec_from_list(arena, *cdr.borrow())?;
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
                },
                _ => break,
            },
            _ => break,
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
    value: usize,
) -> Result<(RcEnv, usize), String> {
    match arena.get(value) {
        Value::SyntacticClosure {
            closed_env,
            free_variables,
            expr,
        } => {
            let closed_env = arena
                .try_get_environment(*closed_env.borrow())
                .expect("Syntactic closure created with non-environment argument.");
            let inner_env = environment::filter(closed_env, env, free_variables)?;
            resolve_syntactic_closure(arena, &inner_env, *expr)
        }
        _ => Ok((env.clone(), value)),
    }
}

fn strip_syntactic_closure(arena: &Arena, value: usize) -> usize {
    match arena.get(value) {
        Value::SyntacticClosure { expr, .. } => strip_syntactic_closure(arena, *expr),
        _ => value,
    }
}

/// Returns the largest toplevel reference encountered in the tree, if any.
pub fn largest_toplevel_reference(se: &SyntaxElement) -> Option<usize> {
    match se {
        SyntaxElement::Reference(r) => match (r.altitude, r.index) {
            (0, i) => Some(i),
            _ => None,
        },
        SyntaxElement::Lambda(l) => {
            let largest_in_defines = l
                .defines
                .iter()
                .map(largest_toplevel_reference)
                .fold(None, max_optional);
            let largest_in_body = l
                .expressions
                .iter()
                .map(largest_toplevel_reference)
                .fold(None, max_optional);
            max_optional(largest_in_defines, largest_in_body)
        }
        SyntaxElement::Begin(b) => b
            .expressions
            .iter()
            .map(largest_toplevel_reference)
            .fold(None, max_optional),
        SyntaxElement::Set(s) => {
            let r = if s.altitude == 0 { Some(s.index) } else { None };
            max_optional(r, largest_toplevel_reference(&s.value))
        }
        SyntaxElement::Quote(_) => None,
        SyntaxElement::If(i) => max_optional(
            largest_toplevel_reference(&i.cond),
            max_optional(
                largest_toplevel_reference(&i.t),
                (&i.f).as_ref().and_then(largest_toplevel_reference),
            ),
        ),
        SyntaxElement::Application(a) => {
            let largest_in_args = a
                .args
                .iter()
                .map(largest_toplevel_reference)
                .fold(None, max_optional);
            max_optional(largest_toplevel_reference(&a.function), largest_in_args)
        }
    }
}
