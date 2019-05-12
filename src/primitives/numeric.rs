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
use util::check_len;
use value::{pretty_print, Value};

/// Generates a numeric primitive that runs a simple fold. The provided folder must be a function
/// (&Value, &Value) -> Value
macro_rules! prim_fold_0 {
    ($name:ident, $folder:ident, $fold_initial:expr) => {
        pub fn $name(arena: &Arena, args: &[usize]) -> Result<usize, String> {
            let values = numeric_vec(arena, args)?;
            Ok(arena.insert(values.iter().fold($fold_initial, |a, b| $folder(&a, &b))))
        }
    };
}

prim_fold_0!(add, add2, Value::Integer(0));
fn add2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia + ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa + fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_fold_0!(mul, mul2, Value::Integer(1));
fn mul2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia * ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa * fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

/// TODO: noncompliant: (- 1) shoud return -1, not 1.
/// Like [prim_fold_0], but uses the first element of the list as the fold initializer
macro_rules! prim_fold_1 {
    ($name:ident, $folder:ident) => {
        pub fn $name(arena: &Arena, args: &[usize]) -> Result<usize, String> {
            let values: Vec<Value> = numeric_vec(arena, args)?;
            check_len(&values, Some(1), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            // TODO can rewrite the thing below without cloning
            let first = values
                .first()
                .expect("with_check_len is broken my bois")
                .clone();
            Ok(arena.insert(values[1..].iter().fold(first, |a, b| $folder(&a, &b))))
        }
    };
}

prim_fold_1!(subn, sub2);
fn sub2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia - ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa - fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

pub fn sub(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    if args.len() == 1 {
        let result = match arena.get(args[0]) {
            Value::Integer(i) => Value::Integer(-i),
            Value::Real(f) => Value::Real(-f),
            _ => {
                return Err(format!(
                    "(-): non-numeric argument: {}",
                    pretty_print(arena, args[0])
                ))
            }
        };
        Ok(arena.insert(result))
    } else {
        subn(arena, args)
    }
}

prim_fold_1!(divn, div2);
fn div2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => {
            if ia % ib == 0 {
                Value::Integer(ia / ib)
            } else {
                Value::Real(ia as f64 / ib as f64)
            }
        }
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa / fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

pub fn div(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    if args.len() == 1 {
        let result = match arena.get(args[0]) {
            Value::Integer(i) => Value::Real(1.0 / (*i) as f64),
            Value::Real(f) => Value::Real(1.0 / (*f)),
            _ => {
                return Err(format!(
                    "(/): non-numeric argument: {}",
                    pretty_print(arena, args[0])
                ))
            }
        };
        Ok(arena.insert(result))
    } else {
        divn(arena, args)
    }
}

/// Generates a numeric primitive that verifies monotonicity. Needs a (&Value, &Value) -> bool
/// function to wrap.
macro_rules! prim_monotonic {
    ($name:ident, $pair:ident) => {
        pub fn $name(arena: &Arena, args: &[usize]) -> Result<usize, String> {
            let values: Vec<Value> = numeric_vec(arena, args)?;
            check_len(&values, Some(2), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            let ans = values.windows(2).all(|x| $pair(&x[0], &x[1]));
            Ok(arena.insert(Value::Boolean(ans)))
        }
    };
}

prim_monotonic!(equal, equal2);
fn equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia == ib,
        (Value::Real(fa), Value::Real(fb)) => (fa - fb).abs() < std::f64::EPSILON,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(less_than, less_than2);
fn less_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia > ib,
        (Value::Real(fa), Value::Real(fb)) => fa > fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(greater_than, greater_than2);
fn greater_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia < ib,
        (Value::Real(fa), Value::Real(fb)) => fa < fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(less_than_equal, less_than_equal2);
fn less_than_equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia <= ib,
        (Value::Real(fa), Value::Real(fb)) => fa <= fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(greater_than_equal, greater_than_equal2);
fn greater_than_equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia >= ib,
        (Value::Real(fa), Value::Real(fb)) => fa >= fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

pub fn integer_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Integer(_) => arena.t,
        _ => arena.f,
    })
}

pub fn real_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Real(_) => arena.t,
        _ => arena.f,
    })
}

/// Takes an argument list (vector of arena pointers), returns a vector of numeric values or
/// an error.
///
/// TODO: we probably shouldn't collect into a vector because it makes basic math slow af.
fn numeric_vec(arena: &Arena, args: &[usize]) -> Result<Vec<Value>, String> {
    args.iter()
        .map(|v| verify_numeric(arena.get(*v).clone()))
        .collect::<Result<Vec<_>, _>>()
}

/// Checks that a value is numeric
fn verify_numeric(a: Value) -> Result<Value, String> {
    match a {
        Value::Integer(_) => Ok(a),
        Value::Real(_) => Ok(a),
        _ => Err(format!("{} is not numeric.", a)),
    }
}

/// Casts two numeric values to the same type.
fn cast_same(a: &Value, b: &Value) -> (Value, Value) {
    match (a, b) {
        (&Value::Integer(ia), &Value::Integer(ib)) => (Value::Integer(ia), Value::Integer(ib)),
        _ => (as_float(a), as_float(b)),
    }
}

/// Turns a numeric value into a float.
fn as_float(n: &Value) -> Value {
    match *n {
        Value::Integer(i) => Value::Real(i as f64),
        Value::Real(f) => Value::Real(f),
        _ => panic!("Non-numeric type passed to as_float"),
    }
}
