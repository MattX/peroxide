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

use std::ops::Neg;

use num_bigint::BigInt;
use num_complex::Complex;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

use arena::Arena;
use util::{check_len, rational_to_f64, simplify_numeric};
use value::{pretty_print, Value};

macro_rules! simple_operator {
    ($inner_name:ident, $operator:tt) => {
        fn $inner_name(a: &Value, b: &Value) -> Value {
            match cast_same(a, b) {
                (Value::ComplexReal(a), Value::ComplexReal(b)) => Value::ComplexReal(a $operator b),
                (Value::ComplexRational(a), Value::ComplexRational(b)) => {
                    Value::ComplexRational(Box::new((*a) $operator (*b)))
                }
                (Value::ComplexInteger(a), Value::ComplexInteger(b)) => {
                    Value::ComplexInteger(Box::new((*a) $operator (*b)))
                }
                (Value::Real(a), Value::Real(b)) => Value::Real(a $operator b),
                (Value::Rational(a), Value::Rational(b)) => {
                    Value::Rational(Box::new((*a) $operator (*b)))
                }
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a $operator b),
                _ => panic!(
                    "cast_same did not return equal numeric types: ({}, {})",
                    a, b
                ),
            }
        }
    }
}

macro_rules! unary_operation {
    ($target:expr, $pp: expr, $operator:ident) => {
        match $target {
            Value::ComplexReal(a) => Value::ComplexReal($operator(a)),
            Value::ComplexRational(a) => Value::ComplexRational(Box::new($operator(&*a))),
            Value::ComplexInteger(a) => Value::ComplexInteger(Box::new($operator(&*a))),
            Value::Real(a) => Value::Real($operator(a)),
            Value::Rational(a) => Value::Rational(Box::new($operator(&*a))),
            Value::Integer(a) => Value::Integer($operator(a)),
            _ => return Err(format!("non-numeric argument: {}", $pp)),
        }
    };
}

/// Generates a numeric primitive that runs a simple fold. The provided folder must be a function
/// (&Value, &Value) -> Value
macro_rules! prim_fold_0 {
    ($name:ident, $folder:ident, $fold_initial:expr) => {
        pub fn $name(arena: &Arena, args: &[usize]) -> Result<usize, String> {
            let values = numeric_vec(arena, args)?;
            let result = values.iter().fold($fold_initial, |a, b| $folder(&a, &b));
            Ok(arena.insert(simplify_numeric(result)))
        }
    };
}

simple_operator!(add2, +);
prim_fold_0!(add, add2, Value::Integer(BigInt::zero()));

simple_operator!(mul2, *);
prim_fold_0!(mul, mul2, Value::Integer(BigInt::one()));

simple_operator!(sub2, -);

fn subn(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    let values = numeric_vec(arena, args)?;
    check_len(&values, Some(1), None).map_err(|e| format!("(-): {}", e))?;
    let first = (*values.first().expect("check_len is broken my bois")).clone();
    let result = values[1..].iter().fold(first, |a, b| sub2(&a, &b));
    Ok(arena.insert(simplify_numeric(result)))
}

fn sub1<'a, T>(n: &'a T) -> T
where
    &'a T: Neg<Output = T>,
{
    -n
}

pub fn sub(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    if args.len() == 1 {
        let result = unary_operation!(arena.get(args[0]), pretty_print(arena, args[0]), sub1);
        Ok(arena.insert(result))
    } else {
        subn(arena, args)
    }
}

fn divn(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    let values = numeric_vec(arena, args)?;
    check_len(&values, Some(1), None).map_err(|e| format!("(/): {}", e))?;
    let mut result = (*values.first().expect("check_len is broken my bois")).clone();
    for v in values[1..].iter() {
        result = div2(&result, v).ok_or_else(|| "exact division by zero".to_string())?;
    }
    Ok(arena.insert(simplify_numeric(result)))
}

fn div2(a: &Value, b: &Value) -> Option<Value> {
    match cast_same(a, b) {
        (Value::ComplexReal(a), Value::ComplexReal(b)) => Some(Value::ComplexReal(a / b)),
        (Value::ComplexRational(a), Value::ComplexRational(b)) => {
            if b.is_zero() {
                None
            } else {
                Some(Value::ComplexRational(Box::new(*a / *b)))
            }
        }
        (Value::ComplexInteger(a), Value::ComplexInteger(b)) => {
            if b.is_zero() {
                None
            } else {
                let rational_a =
                    Complex::<BigRational>::new(a.re.clone().into(), a.im.clone().into());
                let rational_b =
                    Complex::<BigRational>::new(b.re.clone().into(), b.im.clone().into());
                Some(Value::ComplexRational(Box::new(rational_a / rational_b)))
            }
        }
        (Value::Real(a), Value::Real(b)) => Some(Value::Real(a / b)),
        (Value::Rational(a), Value::Rational(b)) => {
            if b.is_zero() {
                None
            } else {
                Some(Value::Rational(Box::new(*a / *b)))
            }
        }
        (Value::Integer(a), Value::Integer(b)) => {
            if b.is_zero() {
                None
            } else {
                Some(Value::Rational(Box::new(BigRational::new(a, b))))
            }
        }
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

pub fn div(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    if args.len() == 1 {
        let arg = arena.get(args[0]);
        if !is_numeric(arg) {
            return Err(format!(
                "non-numeric argument: {}",
                pretty_print(arena, args[0])
            ));
        }
        let result =
            div2(&Value::Integer(BigInt::one()), arg).ok_or_else(|| "division by 0".to_string())?;
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

fn real_part(v: &Value) -> Value {
    match v {
        Value::ComplexReal(a) => Value::Real(a.re),
        Value::ComplexRational(a) => Value::Rational(Box::new(a.re.clone())),
        Value::ComplexInteger(a) => Value::Integer(a.re.clone()),
        Value::Real(_) => v.clone(),
        Value::Rational(_) => v.clone(),
        Value::Integer(_) => v.clone(),
        _ => panic!("real_part: non-numeric value"),
    }
}

fn imag_part(v: &Value) -> Value {
    match v {
        Value::ComplexReal(a) => Value::Real(a.im),
        Value::ComplexRational(a) => Value::Rational(Box::new(a.im.clone())),
        Value::ComplexInteger(a) => Value::Integer(a.re.clone()),
        Value::Real(_) => Value::Real(0.0),
        Value::Rational(_) => Value::Rational(Box::new(BigRational::zero())),
        Value::Integer(_) => Value::Integer(BigInt::zero()),
        _ => panic!("real_part: non-numeric value"),
    }
}

pub fn number_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(is_numeric(arena.get(args[0])))))
}

pub fn real_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Real(_) => arena.t,
        _ => arena.f,
    })
}

pub fn exact_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = match arena.get(args[0]) {
        Value::ComplexRational(_)
        | Value::ComplexInteger(_)
        | Value::Rational(_)
        | Value::Integer(_) => true,
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(resp)))
}

pub fn nan_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = match arena.get(args[0]) {
        Value::ComplexReal(c) => c.is_nan(),
        Value::Real(r) => r.is_nan(),
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(resp)))
}

pub fn infinite_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = match arena.get(args[0]) {
        Value::ComplexReal(c) => c.is_infinite(),
        Value::Real(r) => r.is_infinite(),
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(resp)))
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
            Value::ComplexInteger(Box::new(Complex::new(x.clone(), BigInt::zero())))
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
        Value::ComplexRational(x) => {
            Value::ComplexReal(Complex::new(rational_to_f64(&x.re), rational_to_f64(&x.im)))
        }
        Value::ComplexInteger(x) => {
            Value::ComplexReal(Complex::new(bigint_to_f64(&x.re), bigint_to_f64(&x.im)))
        }
        Value::Real(f) => Value::Real(*f),
        Value::Rational(x) => Value::Real(rational_to_f64(x)),
        Value::Integer(i) => Value::Real(bigint_to_f64(i)),
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
        Value::Integer(i) => Value::Rational(Box::new(BigRational::new(i.clone(), BigInt::one()))),
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
