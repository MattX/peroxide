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
use num_bigint::BigInt;
use num_complex::Complex;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use util::{check_len, rational_to_float};
use value::{pretty_print, Value};

macro_rules! simple_operator {
    ($inner_name:ident, $operator:tt) => {
        fn $inner_name(a: &Value, b: &Value) -> Value {
            match cast_same(a, b) {
                (Value::ComplexReal(a), Value::ComplexReal(b)) => Value::ComplexReal(a $tt b),
                (Value::Real(a), Value::Real(b)) => Value::Real(a $tt b),
                (Value::Rational(a), Value::Rational(b)) => Value::Rational(Box::new(a $tt b)),
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a $tt b),
                _ => panic!(
                    "cast_same did not return equal numeric types: ({}, {})",
                    a, b
                ),
            }
        }
    }
}

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
        (Value::ComplexReal(a), Value::ComplexReal(b)) => Value::ComplexReal(a + b),
        (Value::ComplexRational(a), Value::ComplexRational(b)) => {
            Value::ComplexRational(Box::new(*a + *b))
        }
        (Value::ComplexInteger(a), Value::ComplexInteger(b)) => {
            Value::ComplexInteger(Box::new(*a + *b))
        }
        (Value::Real(a), Value::Real(b)) => Value::Real(a + b),
        (Value::Rational(a), Value::Rational(b)) => Value::Rational(Box::new(*a + *b)),
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
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

/// Like [prim_fold_0], but uses the first element of the list as the fold initializer
macro_rules! prim_fold_1 {
    ($name:ident, $folder:ident) => {
        pub fn $name(arena: &Arena, args: &[usize]) -> Result<usize, String> {
            let values = numeric_vec(arena, args)?;
            check_len(&values, Some(1), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            let first = (*values.first().expect("with_check_len is broken my bois")).clone();
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
            Value::Integer(i) => Value::Integer(-*i),
            Value::Real(f) => Value::Real(-*f),
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
            let values = numeric_vec(arena, args)?;
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
        (Value::Integer(ia), Value::Integer(ib)) => ia < ib,
        (Value::Real(fa), Value::Real(fb)) => fa < fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(greater_than, greater_than2);
fn greater_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia > ib,
        (Value::Real(fa), Value::Real(fb)) => fa > fb,
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
fn numeric_vec<'a>(arena: &'a Arena, args: &[usize]) -> Result<Vec<&'a Value>, String> {
    for arg in args {
        if !is_numeric(arena.get(*arg)) {
            return Err(format!("{} is not numeric.", arg));
        }
    }
    Ok(args.iter().map(|v| arena.get(*v)).collect())
}

/// Checks that a value is numeric
fn is_numeric(a: &Value) -> bool {
    match a {
        Value::Integer(_) => true,
        Value::Rational(_) => true,
        Value::Real(_) => true,
        Value::ComplexInteger(_) => true,
        Value::ComplexRational(_) => true,
        Value::ComplexReal(_) => true,
        _ => false,
    }
}

fn is_complex(a: &Value) -> bool {
    match a {
        Value::ComplexInteger(_) => true,
        Value::ComplexRational(_) => true,
        Value::ComplexReal(_) => true,
        _ => false,
    }
}

fn is_real(a: &Value) -> bool {
    match a {
        Value::ComplexReal(_) | Value::Real(_) => true,
        _ => false,
    }
}

fn is_rational(a: &Value) -> bool {
    match a {
        Value::ComplexRational(_) | Value::Rational(_) => true,
        _ => false,
    }
}

fn is_integer(a: &Value) -> bool {
    match a {
        Value::ComplexInteger(_) | Value::Integer(_) => true,
        _ => false,
    }
}

/// Casts two numeric values to the same type.
fn cast_same(a: &Value, b: &Value) -> (Value, Value) {
    let (a, b) = if is_complex(a) || is_complex(b) {
        (as_complex(a), as_complex(b))
    } else {
        (a.clone(), b.clone())
    };
    if is_real(&a) || is_real(&b) {
        (as_real(&a), as_real(&b))
    } else if is_rational(&a) || is_rational(&b) {
        (as_rational(&a), as_rational(&b))
    } else {
        // Integer
        (a, b)
    }
}

// TODO all the casting methods below are repetitive and not very type-safe. Is there a better
//      way? dun dun dun dun
// TODO the methods below should probably take their arguments by value?

fn as_complex(v: &Value) -> Value {
    match v {
        Value::ComplexReal(_) | Value::ComplexRational(_) | Value::ComplexInteger(_) => v.clone(),
        Value::Real(x) => Value::ComplexReal(Complex::new(*x, 0.0)),
        Value::Integer(x) => {
            Value::ComplexInteger(Box::new(Complex::new((*x).into(), BigInt::zero())))
        }
        Value::Rational(x) => {
            Value::ComplexRational(Box::new(Complex::new(*x.clone(), BigRational::zero())))
        }
        _ => panic!("casting non-number as complex: {:?}", v),
    }
}

fn as_real(n: &Value) -> Value {
    match n {
        Value::ComplexReal(x) => Value::ComplexReal(*x),
        Value::ComplexRational(x) => Value::ComplexReal(Complex::new(
            rational_to_float(&x.re),
            rational_to_float(&x.im),
        )),
        Value::ComplexInteger(x) => {
            Value::ComplexReal(Complex::new(bigint_to_f64(&x.re), bigint_to_f64(&x.im)))
        }
        Value::Real(f) => Value::Real(*f),
        Value::Rational(x) => Value::Real(rational_to_float(x)),
        Value::Integer(i) => Value::Real(*i as f64),
        _ => panic!("cannot cast to float: {:?}", n),
    }
}

fn as_rational(n: &Value) -> Value {
    match n {
        Value::ComplexRational(_) => n.clone(),
        Value::ComplexInteger(x) => Value::ComplexRational(Box::new(Complex::new(
            bigint_to_rational(&x.re),
            bigint_to_rational(&x.im),
        ))),
        Value::Rational(_) => n.clone(),
        Value::Integer(i) => {
            Value::Rational(Box::new(BigRational::new((*i).into(), BigInt::one())))
        }
        _ => panic!("cannot cast to rational: {:?}", n),
    }
}

fn bigint_to_f64(b: &BigInt) -> f64 {
    b.to_f64().unwrap_or_else(|| {
        if b.is_positive() {
            std::f64::INFINITY
        } else {
            std::f64::NEG_INFINITY
        }
    })
}

fn bigint_to_rational(b: &BigInt) -> BigRational {
    BigRational::new(b.clone(), BigInt::one())
}
