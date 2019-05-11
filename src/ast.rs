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
//! Two things we don't support right now and will probably need to:
//!  * Macro support. Besides the macroexpansion processor, we need to keep track of which macros
//!    have been declared in the branch we are in as we construct the tree. This also means
//!    two-way communication with the caller for toplevel macro defines.
//!    I'm also not sure how to handle hygiene for macros.
//!  * A similar but simpler concern is keeping track of any keywords that have been redefined.
//!
//! Once the data has been read, we can drop all of the code we've read and keep only the quotes.
//! I think the easiest way to do this would be to use two separate arenas for the pre-AST and
//! post-AST values.
//!
//! Another small thing is dealing with recursive trees as allowed by R7RS.

use arena::Arena;
use environment::{Environment, EnvironmentValue, RcEnv};
use std::cell::RefCell;
use std::rc::Rc;
use util::check_len;
use value::Value;

#[derive(Debug)]
pub enum SyntaxElement {
    Reference(Box<Reference>),
    Quote(Box<Quote>),
    If(Box<If>),
    Begin(Box<Begin>),
    Lambda(Box<Lambda>),
    Define(Box<Define>),
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
#[derive(Debug)]
pub struct Lambda {
    pub env: RcEnv,
    pub defines: Vec<SyntaxElement>,
    pub expressions: Vec<SyntaxElement>,
}

#[derive(Debug)]
pub struct Define {
    pub altitude: usize,
    pub index: usize,
    pub value: SyntaxElement,
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

pub fn to_syntax_element(
    arena: &Arena,
    env: &RcEnv,
    value: usize,
    toplevel: bool,
) -> Result<SyntaxElement, String> {
    match arena.get(value) {
        Value::Symbol(s) => Ok(SyntaxElement::Reference(Box::new(construct_reference(
            env, s,
        )?))),
        Value::EmptyList => Err("Cannot evaluate empty list".into()),
        Value::Pair(car, cdr) => {
            pair_to_syntax_element(arena, env, *car.borrow(), *cdr.borrow(), toplevel)
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
            Ok(Reference { altitude: 0, index })
        }
    }
}

fn pair_to_syntax_element(
    arena: &Arena,
    env: &RcEnv,
    car: usize,
    cdr: usize,
    toplevel: bool,
) -> Result<SyntaxElement, String> {
    let rest = arena.get(cdr).pair_to_vec(arena)?;
    match arena.get(car) {
        Value::Symbol(s) => match s.as_ref() {
            "quote" => parse_quote(&rest),
            "if" => parse_if(arena, env, &rest),
            "begin" => parse_begin(arena, env, &rest),
            "lambda" => parse_lambda(arena, env, &rest),
            "set!" => parse_set(arena, env, &rest),
            "define" => parse_define(arena, env, &rest, toplevel),
            _ => parse_application(arena, env, car, &rest),
        },
        _ => parse_application(arena, env, car, &rest),
    }
}

fn parse_quote(rest: &[usize]) -> Result<SyntaxElement, String> {
    if rest.len() != 1 {
        Err(format!("quote expected 1 argument, got {}.", rest.len()))
    } else {
        Ok(SyntaxElement::Quote(Box::new(Quote { quoted: rest[0] })))
    }
}

fn parse_if(arena: &Arena, env: &RcEnv, rest: &[usize]) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(3))?;
    let cond = to_syntax_element(arena, env, rest[0], false)?;
    let t = to_syntax_element(arena, env, rest[1], false)?;
    let f_s: Option<Result<_, _>> = rest
        .get(2)
        .map(|e| to_syntax_element(arena, env, *e, false));

    // This dark magic swaps the option and the result (then `?`s the result)
    // https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/option_result.html
    let f: Option<_> = f_s.map_or(Ok(None), |r| r.map(Some))?;
    Ok(SyntaxElement::If(Box::new(If { cond, t, f })))
}

fn parse_begin(arena: &Arena, env: &RcEnv, rest: &[usize]) -> Result<SyntaxElement, String> {
    check_len(rest, Some(1), None)?;
    let expressions = rest
        .iter()
        .map(|e| to_syntax_element(arena, env, *e, false))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Begin(Box::new(Begin { expressions })))
}

fn parse_lambda(arena: &Arena, env: &RcEnv, rest: &[usize]) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), None)?;
    parse_split_lambda(arena, env, rest[0], &rest[1..rest.len()])
}

fn parse_split_lambda(
    arena: &Arena,
    env: &RcEnv,
    formals: usize,
    body: &[usize],
) -> Result<SyntaxElement, String> {
    if body.is_empty() {
        return Err("Lambda cannot have empty body.".into());
    }
    let formals = parse_formals(arena, formals)?;
    let mut raw_env = Environment::new_initial(Some(env.clone()), &formals.values[..]);
    if let Some(s) = &formals.rest {
        raw_env.define(s, true);
    }
    let env = Rc::new(RefCell::new(raw_env));
    let expressions = body
        .iter()
        .map(|e| to_syntax_element(arena, &env, *e, false))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SyntaxElement::Lambda(Box::new(Lambda {
        env,
        defines: vec![],
        expressions,
    })))
}

fn parse_set(arena: &Arena, env: &RcEnv, rest: &[usize]) -> Result<SyntaxElement, String> {
    check_len(rest, Some(2), Some(2))?;
    if let Value::Symbol(name) = arena.get(rest[0]) {
        let value = to_syntax_element(arena, env, rest[1], false)?;
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
            arena.get(rest[0]).pretty_print(arena)
        ))
    }
}

enum DefineValue {
    Value(usize),
    Lambda { formals: usize, body: Vec<usize> },
}

/// Parses toplevel defines. Inner defines have different semantics and will need to be parsed
/// elsewhere.
fn parse_define(
    arena: &Arena,
    env: &RcEnv,
    rest: &[usize],
    toplevel: bool,
) -> Result<SyntaxElement, String> {
    if !toplevel {
        return Err("Define in illegal position.".into());
    }
    let (symbol, define_value) = match arena.get(rest[0]) {
        Value::Symbol(s) => {
            check_len(rest, Some(2), Some(2))?;
            (s.clone(), DefineValue::Value(rest[1]))
        }
        _ => parse_lambda_define(arena, env, rest)?,
    };
    let index = env.borrow_mut().define_if_absent(&symbol, false);
    let value = match define_value {
        DefineValue::Value(v) => to_syntax_element(arena, env, v, false)?,
        DefineValue::Lambda { formals, body } => parse_split_lambda(arena, env, formals, &body)?,
    };
    Ok(SyntaxElement::Define(Box::new(Define {
        altitude: 0,
        index,
        value,
    })))
}

/// Helper method to parse direct lambda defines `(define (x y z) y z)`.
fn parse_lambda_define(
    arena: &Arena,
    env: &RcEnv,
    rest: &[usize],
) -> Result<(String, DefineValue), String> {
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
                arena.get(*car.borrow()).pretty_print(arena)
            ))
        }
    } else {
        Err(format!(
            "Expected symbol or formals as target of define, got `{}`.",
            arena.get(rest[0]).pretty_print(arena)
        ))
    }
}

fn parse_application(
    arena: &Arena,
    env: &RcEnv,
    fun: usize,
    rest: &[usize],
) -> Result<SyntaxElement, String> {
    let function = to_syntax_element(arena, env, fun, false)?;
    let args = rest
        .iter()
        .map(|e| to_syntax_element(arena, env, *e, false))
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
        match arena.get(formal) {
            Value::Symbol(s) => {
                return Ok(Formals {
                    values,
                    rest: Some(s.clone()),
                });
            }
            Value::EmptyList => return Ok(Formals { values, rest: None }),
            Value::Pair(car, cdr) => {
                if let Value::Symbol(s) = arena.get(*car.borrow()) {
                    values.push(s.clone());
                    formal = *cdr.borrow();
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

fn collect_defines(
    body: &[SyntaxElement],
) -> Result<(Vec<SyntaxElement>, &[SyntaxElement]), String> {
    unimplemented!()
}
