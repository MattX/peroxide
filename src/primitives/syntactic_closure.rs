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

use std::cell::RefCell;
use std::rc::Rc;

use arena::Arena;
use environment;
use environment::{Environment, EnvironmentValue, RcEnv};
use heap::PoolPtr;
use util::check_len;
use value::{list_from_vec, Value};

#[derive(Debug, PartialEq, Clone)]
pub struct SyntacticClosure {
    pub closed_env: RefCell<PoolPtr>,
    pub free_variables: Vec<String>,
    pub expr: PoolPtr,
}

impl SyntacticClosure {
    pub fn push_env(&self, arena: &Arena) -> RcEnv {
        let env = self
            .closed_env
            .borrow()
            .try_get_environment()
            .expect("syntactic closure created with non-env")
            .clone();
        let inner_env = Rc::new(RefCell::new(Environment::new(Some(env))));
        let inner_env_val = Value::Environment(inner_env.clone());
        RefCell::replace(&self.closed_env, arena.insert(inner_env_val));
        inner_env
    }

    pub fn pop_env(&self, arena: &Arena) {
        let parent_env = self
            .closed_env
            .borrow()
            .try_get_environment()
            .expect("syntactic closure created with non-env")
            .borrow()
            .parent()
            .expect("Popping from syntactic closure with no parent env.")
            .clone();
        RefCell::replace(
            &self.closed_env,
            arena.insert(Value::Environment(parent_env)),
        );
    }
}

pub fn make_syntactic_closure(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(3), Some(3))?;
    let free_variables = args[1]
        .list_to_vec()?
        .iter()
        .map(|fv| match &**fv {
            Value::Symbol(s) => Ok(s.clone()),
            _ => Err(format!(
                "make-syntactic-closure: not a symbol: {}",
                fv.pretty_print()
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let closed_env = match &*args[0] {
        Value::Environment(_) => Ok(args[0]),
        _ => Err(format!(
            "make-syntactic-closure: not an environment: {}",
            args[0].pretty_print()
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
fn get_in_env(env: &RcEnv, val: PoolPtr) -> Result<Option<EnvironmentValue>, String> {
    match &*val {
        Value::Symbol(s) => Ok(env.borrow().get(s)),
        Value::SyntacticClosure(SyntacticClosure {
            closed_env,
            free_variables,
            expr,
        }) => {
            let borrow = closed_env.borrow();
            let closed_env = borrow
                .try_get_environment()
                .expect("Syntactic closure created with non-environment argument.");
            let inner_env = environment::filter(closed_env, env, free_variables)?;
            get_in_env(&inner_env, *expr)
        }
        _ => Err(format!("non-identifier: {}", val.pretty_print())),
    }
}

pub fn identifier_equal_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(4), Some(4))?;
    let env1 = args[0].try_get_environment().ok_or_else(|| {
        format!(
            "identifier=?: not an environment: {}",
            args[0].pretty_print()
        )
    })?;
    let env2 = args[2].try_get_environment().ok_or_else(|| {
        format!(
            "identifier=?: not an environment: {}",
            args[2].pretty_print()
        )
    })?;

    if !is_identifier(args[1]) || !is_identifier(args[3]) {
        return Ok(arena.f);
    }

    let binding1 = get_in_env(env1, args[1])?;
    let binding2 = get_in_env(env2, args[3])?;

    let res = match (binding1, binding2) {
        (None, None) => coerce_symbol(args[1]) == coerce_symbol(args[3]),
        (Some(EnvironmentValue::Variable(v1)), Some(EnvironmentValue::Variable(v2))) => {
            v1.altitude == v2.altitude && v1.index == v2.index
        }
        (Some(EnvironmentValue::Macro(m1)), Some(EnvironmentValue::Macro(m2))) => {
            // Lambdas are unique so no need to check environment equality
            m1.lambda.pp() == m2.lambda.pp()
        }
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(res)))
}

fn coerce_symbol(value: PoolPtr) -> String {
    match &*value {
        Value::Symbol(s) => s.clone(),
        Value::SyntacticClosure(sc) => coerce_symbol(sc.expr),
        _ => panic!(
            "Coercing non-identifier {} to symbol.",
            value.pretty_print()
        ),
    }
}

fn is_identifier(value: PoolPtr) -> bool {
    match &*value {
        Value::Symbol(_) => true,
        Value::SyntacticClosure(SyntacticClosure { expr, .. }) => is_identifier(*expr),
        _ => false,
    }
}

pub fn identifier_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(is_identifier(args[0]))))
}

pub fn syntactic_closure_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(
        args[0].try_get_syntactic_closure().is_some(),
    )))
}

pub fn syntactic_closure_environment(_arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let synclos = args[0]
        .try_get_syntactic_closure()
        .ok_or_else(|| format!("not a syntactic closure: {}", args[0].pretty_print()))?;
    Ok(*synclos.closed_env.borrow())
}

pub fn syntactic_closure_free_variables(
    arena: &Arena,
    args: &[PoolPtr],
) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let synclos = args[0]
        .try_get_syntactic_closure()
        .ok_or_else(|| format!("not a syntactic closure: {}", args[0].pretty_print()))?;
    let symbols = synclos
        .free_variables
        .iter()
        .map(|s| arena.insert(Value::Symbol(s.clone())));
    let sv: Vec<_> = symbols.collect();
    Ok(list_from_vec(arena, &sv))
}

pub fn syntactic_closure_expression(_arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let synclos = args[0]
        .try_get_syntactic_closure()
        .ok_or_else(|| format!("not a syntactic closure: {}", args[0].pretty_print()))?;
    Ok(synclos.expr)
}

pub fn gensym(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(0), Some(1))?;
    let base_name = if let Some(v) = args.get(0) {
        Some(
            v.try_get_string()
                .map(|s| s.borrow().clone())
                .ok_or_else(|| format!("not a string: {}", v.pretty_print()))?,
        )
    } else {
        None
    };
    Ok(arena.gensym(base_name.as_deref()))
}
