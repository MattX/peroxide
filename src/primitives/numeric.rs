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
use std::convert::TryFrom;
use std::ops::{Neg, Rem};

use arena::Arena;
use heap::PoolPtr;
use num_bigint::BigInt;
use num_complex::Complex;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{pow, Float, One, Signed, ToPrimitive, Zero};
use util::{check_len, is_numeric, rational_to_f64};
use value::Value;
use {lex, read};

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
        pub fn $name(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
            let values = numeric_vec(args)?;
            let result = values.iter().fold($fold_initial, |a, b| $folder(&a, &b));
            Ok(arena.insert(result))
        }
    };
}

simple_operator!(add2, +);
prim_fold_0!(add, add2, Value::Integer(BigInt::zero()));

simple_operator!(mul2, *);
prim_fold_0!(mul, mul2, Value::Integer(BigInt::one()));

simple_operator!(sub2, -);

fn subn(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let values = numeric_vec(args)?;
    check_len(&values, Some(1), None).map_err(|e| format!("(-): {}", e))?;
    let first = (*values.first().expect("check_len is broken my bois")).clone();
    let result = values[1..].iter().fold(first, |a, b| sub2(&a, b));
    Ok(arena.insert(result))
}

fn sub1<'a, T>(n: &'a T) -> T
where
    &'a T: Neg<Output = T>,
{
    -n
}

pub fn sub(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    if args.len() == 1 {
        let result = unary_operation!(&*args[0], args[0].pretty_print(), sub1);
        Ok(arena.insert(result))
    } else {
        subn(arena, args)
    }
}

fn divn(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let values = numeric_vec(args)?;
    check_len(&values, Some(1), None).map_err(|e| format!("(/): {}", e))?;
    let mut result = (*values.first().expect("check_len is broken my bois")).clone();
    for v in values[1..].iter() {
        result = div2(&result, v).ok_or_else(|| "exact division by zero".to_string())?;
    }
    Ok(arena.insert(result))
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

pub fn div(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    if args.len() == 1 {
        let arg = &*args[0];
        if !is_numeric(arg) {
            return Err(format!("non-numeric argument: {}", args[0].pretty_print()));
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
        pub fn $name(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
            let values = numeric_vec(args)?;
            check_len(&values, Some(2), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            let ans = values.windows(2).all(|x| $pair(&x[0], &x[1]));
            Ok(arena.insert(Value::Boolean(ans)))
        }
    };
}

prim_monotonic!(equal, equal2);
#[allow(clippy::float_cmp)]
fn equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::ComplexInteger(a), Value::ComplexInteger(b)) => a == b,
        (Value::ComplexRational(a), Value::ComplexRational(b)) => a == b,
        (Value::ComplexReal(a), Value::ComplexReal(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Rational(a), Value::Rational(b)) => a == b,
        (Value::Real(fa), Value::Real(fb)) => fa == fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

macro_rules! compare_op {
    ($inner_name:ident, $operator:tt) => {
        fn $inner_name(a: &Value, b: &Value) -> bool {
            match cast_same(a, b) {
                (Value::Real(a), Value::Real(b)) => a $operator b,
                (Value::Rational(a), Value::Rational(b)) => a $operator b,
                (Value::Integer(a), Value::Integer(b)) => a $operator b,
                _ => false,
            }
        }
    }
}

compare_op!(less_than2, <);
prim_monotonic!(less_than, less_than2);

compare_op!(greater_than2, >);
prim_monotonic!(greater_than, greater_than2);

compare_op!(less_than_equal2, <=);
prim_monotonic!(less_than_equal, less_than_equal2);

compare_op!(greater_than_equal2, >=);
prim_monotonic!(greater_than_equal, greater_than_equal2);

pub fn real_part(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match &*args[0] {
        Value::ComplexReal(a) => arena.insert(Value::Real(a.re)),
        Value::ComplexRational(a) => arena.insert(Value::Rational(Box::new(a.re.clone()))),
        Value::ComplexInteger(a) => arena.insert(Value::Integer(a.re.clone())),
        Value::Real(_) => args[0],
        Value::Rational(_) => args[0],
        Value::Integer(_) => args[0],
        _ => {
            return Err(format!(
                "real-part: non-numeric value {}",
                args[0].pretty_print()
            ))
        }
    })
}

pub fn imag_part(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(match &*args[0] {
        Value::ComplexReal(a) => Value::Real(a.im),
        Value::ComplexRational(a) => Value::Rational(Box::new(a.im.clone())),
        Value::ComplexInteger(a) => Value::Integer(a.re.clone()),
        Value::Real(_) => Value::Real(0.0),
        Value::Rational(_) => Value::Rational(Box::new(BigRational::zero())),
        Value::Integer(_) => Value::Integer(BigInt::zero()),
        _ => {
            return Err(format!(
                "real-part: non-numeric value {}",
                args[0].pretty_print()
            ))
        }
    }))
}

pub fn number_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(is_numeric(&*args[0]))))
}

pub fn real_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match &*args[0] {
        Value::Real(_) | Value::Rational(_) | Value::Integer(_) => arena.t,
        _ => arena.f,
    })
}

pub fn rational_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(
        is_integer(&*args[0]) || is_rational(&*args[0]),
    )))
}

pub fn integer_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(is_integer(&*args[0]))))
}

pub fn exact_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = matches!(
        &*args[0],
        Value::ComplexRational(_)
            | Value::ComplexInteger(_)
            | Value::Rational(_)
            | Value::Integer(_)
    );
    Ok(arena.insert(Value::Boolean(resp)))
}

pub fn nan_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = match &*args[0] {
        Value::ComplexReal(c) => c.is_nan(),
        Value::Real(r) => r.is_nan(),
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(resp)))
}

pub fn infinite_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let resp = match &*args[0] {
        Value::ComplexReal(c) => c.is_infinite(),
        Value::Real(r) => r.is_infinite(),
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(resp)))
}

pub fn inexact(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    if !is_numeric(&args[0]) {
        return Err(format!("not a number: {}", args[0].pretty_print()));
    }
    Ok(arena.insert(as_real(&*args[0])))
}

pub fn exact(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    if !is_numeric(&args[0]) {
        return Err(format!("not a number: {}", args[0].pretty_print()));
    }
    Ok(match &*args[0] {
        Value::ComplexReal(c) => arena.insert(Value::ComplexRational(Box::new(Complex::new(
            f64_to_rational(c.re),
            f64_to_rational(c.im),
        )))),
        Value::Real(f) => arena.insert(Value::Rational(Box::new(f64_to_rational(*f)))),
        _ => args[0],
    })
}

pub fn expt(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let result = match (&*args[0], &*args[1]) {
        (Value::Integer(base), Value::Integer(exponent)) => {
            let realistic_exponent = exponent.to_u32();
            realistic_exponent
                .map(|re| Value::Integer(base.pow(re)))
                .unwrap_or(Value::Real(std::f64::INFINITY))
        }
        _ => return Err("only integer exponentiation is supported".to_string()),
    };
    Ok(arena.insert(result))
}

pub fn modulo(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let result = match (&*args[0], &*args[1]) {
        (Value::Integer(dividend), Value::Integer(divisor)) => {
            Value::Integer(dividend.mod_floor(divisor))
        }
        _ => return Err("modulo is only supported for integers".to_string()),
    };
    Ok(arena.insert(result))
}

pub fn remainder(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let result = match (&*args[0], &*args[1]) {
        (Value::Integer(dividend), Value::Integer(divisor)) => {
            Value::Integer(dividend.rem(divisor))
        }
        _ => return Err("remainder is only supported for integers".to_string()),
    };
    Ok(arena.insert(result))
}

pub fn gcd(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let mut acc = BigInt::zero();
    for arg in args {
        if let Value::Integer(i) = &**arg {
            acc = acc.gcd(i);
        } else {
            return Err(format!("non-integer argument: {}", arg.pretty_print()));
        }
    }
    Ok(arena.insert(Value::Integer(acc)))
}

pub fn lcm(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let mut acc = BigInt::one();
    for arg in args {
        if let Value::Integer(i) = &**arg {
            acc = acc.lcm(i);
        } else {
            return Err(format!("non-integer argument: {}", arg.pretty_print()));
        }
    }
    Ok(arena.insert(Value::Integer(acc)))
}

macro_rules! transcendental {
    ($inner_name:ident, $operator:tt) => {
        pub fn $inner_name(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
            check_len(args, Some(1), Some(1))?;
            let arg = &*args[0];
            if !is_numeric(arg) {
                return Err(format!("non-numeric value: {}", args[0].pretty_print()));
            }
            Ok(arena.insert(match as_real(arg) {
                Value::ComplexReal(c) => Value::ComplexReal(c.$operator()),
                Value::Real(c) => Value::Real(c.$operator()),
                _ => panic!("conversion to real failed."),
            }))
        }
    };
}

transcendental!(exp, exp);
transcendental!(log, ln);
transcendental!(cos, cos);
transcendental!(sin, sin);
transcendental!(tan, tan);
transcendental!(acos, acos);
transcendental!(asin, asin);
transcendental!(atan, atan);
transcendental!(sqrt, sqrt);

pub fn magnitude(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    if !is_numeric(&args[0]) {
        return Err(format!("non-numeric value: {}", args[0].pretty_print()));
    }
    Ok(arena.insert(match as_complex(&as_real(&args[0])) {
        Value::ComplexReal(c) => Value::Real(c.norm()),
        _ => panic!("conversion to complex failed."),
    }))
}

pub fn angle(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    if !is_numeric(&args[0]) {
        return Err(format!("non-numeric value: {}", args[0].pretty_print()));
    }
    Ok(arena.insert(match as_complex(&as_real(&args[0])) {
        Value::ComplexReal(c) => Value::Real(c.arg()),
        _ => panic!("conversion to complex failed."),
    }))
}

fn get_radix(v: Option<&PoolPtr>) -> Result<u8, String> {
    let r = match v {
        Some(r) => r
            .try_get_integer()
            .map(|x| x.to_u8().unwrap())
            .ok_or_else(|| format!("invalid radix: {}", r.pretty_print()))?,
        None => 10u8,
    };
    if ![2u8, 8, 10, 16].contains(&r) {
        return Err(format!("radix must be 2, 8, 10 or 16, not {}", r));
    }
    Ok(r)
}

pub fn string_to_number(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(2))?;
    let radix = get_radix(args.get(1))?;
    let st = args[0]
        .try_get_string()
        .ok_or_else(|| format!("invalid argument: {}", args[0].pretty_print()))?;
    let chars = st.borrow().chars().collect::<Vec<_>>();
    let parsed = lex::parse_full_number(&chars, radix)
        .map(|parsed| read::read_num_token(&parsed))
        .map(|read| arena.insert(read))
        .unwrap_or(arena.f);
    Ok(parsed)
}

pub fn number_to_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(2))?;
    let radix = get_radix(args.get(1))? as u32;

    fn format_int(n: &BigInt, r: u32) -> String {
        n.to_str_radix(r)
    }

    fn format_real(n: f64, r: u32) -> Result<String, String> {
        if r != 10 {
            Err("inexact numbers can only be formatted in radix 10.".to_string())
        } else {
            Ok(format!("{}", n))
        }
    }

    fn format_rational(n: &BigRational, r: u32) -> String {
        format!(
            "{}/{}",
            n.numer().to_str_radix(r),
            n.denom().to_str_radix(r)
        )
    }

    let resp = match &*args[0] {
        Value::Integer(a) => format_int(a, radix),
        Value::Real(a) => format_real(*a, radix)?,
        Value::Rational(a) => format_rational(a, radix),
        Value::ComplexReal(a) => format!(
            "{}+{}i",
            format_real(a.re, radix)?,
            format_real(a.im, radix)?
        ),
        Value::ComplexInteger(a) => {
            format!("{}+{}i", format_int(&a.re, radix), format_int(&a.im, radix))
        }
        Value::ComplexRational(a) => format!(
            "{}+{}i",
            format_rational(&a.re, radix),
            format_rational(&a.im, radix)
        ),
        _ => return Err(format!("converting non-number: {}", args[0].pretty_print())),
    };
    Ok(arena.insert(Value::String(RefCell::new(resp))))
}

pub fn make_rectangular(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let re = &*args[0];
    let im = &*args[1];
    if !is_numeric(re) || !is_numeric(im) {
        return Err(format!(
            "not numeric: {}, {}",
            re.pretty_print(),
            im.pretty_print()
        ));
    }
    let res = match cast_same(re, im) {
        (Value::Real(re), Value::Real(im)) => Value::ComplexReal(Complex::new(re, im)),
        (Value::Rational(re), Value::Rational(im)) => {
            Value::ComplexRational(Box::new(Complex::new(*re, *im)))
        }
        (Value::Integer(re), Value::Integer(im)) => {
            Value::ComplexInteger(Box::new(Complex::new(re, im)))
        }
        _ => {
            return Err(format!(
                "arguments must be real: {}, {}",
                args[0].pretty_print(),
                args[1].pretty_print()
            ))
        }
    };
    Ok(arena.insert(res))
}

pub fn make_polar(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let magnitude = &*args[0];
    let angle = &*args[1];
    if !is_numeric(magnitude) || !is_numeric(angle) {
        return Err(format!(
            "not numeric: {}, {}",
            magnitude.pretty_print(),
            angle.pretty_print()
        ));
    }
    let magnitude = as_real(magnitude);
    let angle = as_real(angle);
    if let (Value::Real(magnitude), Value::Real(angle)) = (magnitude, angle) {
        Ok(arena.insert(Value::ComplexReal(Complex::from_polar(magnitude, angle))))
    } else {
        Err(format!(
            "arguments must be real: {}, {}",
            args[0].pretty_print(),
            args[1].pretty_print()
        ))
    }
}

/// Takes an argument list (vector of arena pointers), returns a vector of numeric values or
/// an error.
///
/// TODO: we probably shouldn't collect into a vector because it makes basic math slow af.
fn numeric_vec(args: &[PoolPtr]) -> Result<Vec<&Value>, String> {
    for arg in args {
        if !is_numeric(&*arg) {
            return Err(format!("{} is not numeric", arg.pretty_print()));
        }
    }
    Ok(args.iter().map(|v| &**v).collect())
}

fn is_complex(a: &Value) -> bool {
    matches!(
        a,
        Value::ComplexInteger(_) | Value::ComplexRational(_) | Value::ComplexReal(_)
    )
}

fn is_real(a: &Value) -> bool {
    matches!(a, Value::ComplexReal(_) | Value::Real(_))
}

fn is_rational(a: &Value) -> bool {
    matches!(a, Value::ComplexRational(_) | Value::Rational(_))
}

fn is_integer(a: &Value) -> bool {
    matches!(a, Value::ComplexInteger(_) | Value::Integer(_))
}

/// Casts two numeric values to the same type.
fn cast_same(a: &Value, b: &Value) -> (Value, Value) {
    if !is_numeric(a) || !is_numeric(b) {
        panic!("cast_same called on non-numeric value");
    }
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

fn f64_to_rational(f: f64) -> BigRational {
    let (mantissa, exponent, sign) = f.integer_decode();
    let signed_mantissa = i64::try_from(mantissa).unwrap() * i64::from(sign);
    let mut numer = BigInt::from(signed_mantissa);
    let mut denom = BigInt::one();

    if exponent >= 0 {
        numer *= pow(BigInt::from(2), usize::try_from(exponent).unwrap());
    } else {
        denom *= pow(BigInt::from(2), usize::try_from(-exponent).unwrap())
    }

    BigRational::new(numer, denom)
}
