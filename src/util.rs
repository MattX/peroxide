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

use num_bigint::BigInt;
use num_complex::Complex;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use value::Value;

/// Checks that a vector has at least `min`, at most `max` entries.
// TODO this is not really idiomatic and should probably be made to return a boolean
pub fn check_len<T>(v: &[T], min: Option<usize>, max: Option<usize>) -> Result<(), String> {
    if let Some(m) = min {
        if v.len() < m {
            return Err(format!("Too few values, expecting at least {}.", m));
        }
    };
    if let Some(m) = max {
        if v.len() > m {
            return Err(format!("Too many values, expecting at most {}.", m));
        }
    };
    Ok(())
}

pub fn parse_num(s: &str, base: u32) -> Result<i64, String> {
    if base > 36 {
        panic!("Invalid base {}.", base);
    }

    let mut r = 0 as i64;
    let mut it = s.chars().peekable();
    let reverse = it.peek() == Some(&'-');
    if reverse {
        it.next();
    }

    for d in it {
        let n = d.to_digit(base);
        if let Some(n) = n {
            r = r * i64::from(base) + i64::from(n);
        } else {
            return Err(format!("Invalid digit for base {}: {}", base, d));
        }
    }

    if reverse {
        r = -r;
    }
    Ok(r)
}

pub fn str_to_char_vec(s: &str) -> Vec<char> {
    s.chars().collect()
}

pub fn char_vec_to_str(v: &[char]) -> String {
    v.iter().collect()
}

pub fn escape_char(c: char) -> String {
    match c {
        '\x07' => "alarm".into(),
        '\x08' => "backspace".into(),
        '\x7F' => "delete".into(),
        '\x1B' => "escape".into(),
        '\n' => "newline".into(),
        '\0' => "null".into(),
        '\r' => "return".into(),
        ' ' => "space".into(),
        '\t' => "tab".into(),
        c => c.to_string(),
    }
}

pub fn escape_string(s: &str) -> String {
    let mut output = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\x07' => output.push_str("\\a"),
            '\x08' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '|' => output.push_str("\\|"),
            _ => output.push(c),
        }
    }
    output
}

pub fn escape_symbol(s: &str) -> String {
    if s.is_ascii() {
        s.into()
    } else {
        format!("|{}|", s)
    }
}

// TODO these unwraps suck, and can fail if the bigintegers are large enough. Easy to fix
//      by repeatedly diving each side until they're small.
//      Will be done natively by rust_num once https://github.com/rust-num/num-rational/issues/4
//      is merged.
pub fn rational_to_float(v: &BigRational) -> f64 {
    v.numer().to_f64().unwrap() / v.denom().to_f64().unwrap()
}

pub fn integer_to_float(v: &BigInt) -> f64 {
    v.to_f64().unwrap_or_else(|| {
        if v.is_positive() {
            std::f64::INFINITY
        } else {
            std::f64::NEG_INFINITY
        }
    })
}

fn bigint_to_i64(b: &BigInt) -> i64 {
    let min_i64: BigInt = std::i64::MIN.into();
    let max_i64: BigInt = std::i64::MAX.into();
    let clamped = std::cmp::max(std::cmp::min(b, &max_i64), &min_i64);
    clamped.to_i64().unwrap()
}

/// Turns complex value with an exact zero imaginary part into reals, and integer rationals into
/// proper integers.
pub fn simplify_numeric(v: Value) -> Value {
    let realified = match &v {
        Value::ComplexRational(x) if x.im.is_zero() => {
            Some(Value::Rational(Box::new(x.re.clone())))
        }
        Value::ComplexInteger(x) if x.im.is_zero() => Some(Value::Integer(bigint_to_i64(&x.re))),
        _ => None,
    }
    .unwrap_or(v);
    match &realified {
        Value::Rational(x) if x.is_integer() => {
            Some(Value::Integer(bigint_to_i64(&x.to_integer())))
        }
        Value::ComplexRational(x) => {
            if x.re.is_integer() && x.im.is_integer() {
                Some(Value::ComplexInteger(Box::new(Complex::new(
                    x.re.to_integer(),
                    x.im.to_integer(),
                ))))
            } else {
                None
            }
        }
        _ => None,
    }
    .unwrap_or(realified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_num() {
        assert_eq!(42, parse_num("101010", 2).unwrap());
        assert_eq!(42, parse_num("2a", 16).unwrap());
        assert_eq!(42, parse_num("42", 10).unwrap());
        assert_eq!(-15, parse_num("-F", 16).unwrap());
    }
}
