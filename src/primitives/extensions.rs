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

use arena::Arena;
use environment;
use environment::{EnvironmentValue, RcEnv};
use std::cell::RefCell;
use util::check_len;
use value::{pretty_print, vec_from_list, SyntacticClosure, Value};

pub fn make_syntactic_closure(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(3), Some(3))?;
    let free_variables = vec_from_list(arena, args[1])?
        .iter()
        .map(|fv| match arena.get(*fv) {
            Value::Symbol(s) => Ok(s.clone()),
            _ => Err(format!(
                "make-syntactic-closure: not a symbol: {}",
                pretty_print(arena, *fv)
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let closed_env = match arena.get(args[0]) {
        Value::Environment(_) => Ok(args[0]),
        _ => Err(format!(
            "make-syntactic-closure: not an environment: {}",
            pretty_print(arena, args[0])
        )),
    }?;
    Ok(arena.insert(Value::SyntacticClosure(SyntacticClosure {
        closed_env: RefCell::new(closed_env),
        free_variables,
        expr: args[2],
    })))
}

/// Resolve an identifier in a given environment.
///
/// The outer Result is an error if the passed `val` is not a valid identifier. The inner
/// Option<EnvironmentValue> corresponds to the normal return type for an environment query.
fn get_in_env(arena: &Arena, env: &RcEnv, val: usize) -> Result<Option<EnvironmentValue>, String> {
    match arena.get(val) {
        Value::Symbol(s) => Ok(env.borrow().get(s)),
        Value::SyntacticClosure(SyntacticClosure {
            closed_env,
            free_variables,
            expr,
        }) => {
            let closed_env = arena
                .try_get_environment(*closed_env.borrow())
                .expect("Syntactic closure created with non-environment argument.");
            let inner_env = environment::filter(closed_env, env, free_variables)?;
            get_in_env(arena, &inner_env, *expr)
        }
        _ => Err(format!("Non-identifier: {}", pretty_print(arena, val))),
    }
}

pub fn identifier_equal_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(4), Some(4))?;
    let env1 = arena.try_get_environment(args[0]).ok_or_else(|| {
        format!(
            "identifier=?: not an environment: {}",
            pretty_print(arena, args[0])
        )
    })?;
    let env2 = arena.try_get_environment(args[2]).ok_or_else(|| {
        format!(
            "identifier=?: not an environment: {}",
            pretty_print(arena, args[2])
        )
    })?;

    if !is_identifier(arena, args[1]) || !is_identifier(arena, args[3]) {
        return Ok(arena.f);
    }

    let binding1 = get_in_env(arena, env1, args[1])?;
    let binding2 = get_in_env(arena, env2, args[3])?;

    let res = match (binding1, binding2) {
        (None, None) => coerce_symbol(arena, args[1]) == coerce_symbol(arena, args[3]),
        (Some(EnvironmentValue::Variable(v1)), Some(EnvironmentValue::Variable(v2))) => {
            v1.altitude == v2.altitude && v1.index == v2.index
        }
        (Some(EnvironmentValue::Macro(m1)), Some(EnvironmentValue::Macro(m2))) => {
            // Lambdas are unique so no need to check environment equality
            m1.lambda == m2.lambda
        }
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(res)))
}

fn coerce_symbol(arena: &Arena, value: usize) -> String {
    match arena.get(value) {
        Value::Symbol(s) => s.clone(),
        Value::SyntacticClosure(sc) => coerce_symbol(arena, sc.expr),
        _ => panic!(
            "Coercing non-identifier {} to symbol.",
            pretty_print(arena, value)
        ),
    }
}

fn is_identifier(arena: &Arena, value: usize) -> bool {
    match arena.get(value) {
        Value::Symbol(_) => true,
        Value::SyntacticClosure(SyntacticClosure { expr, .. }) => is_identifier(arena, *expr),
        _ => false,
    }
}

pub fn identifier_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(is_identifier(arena, args[0]))))
}
