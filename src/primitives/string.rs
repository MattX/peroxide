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

use std::cell::{Ref, RefCell};

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use arena::Arena;
use heap::PoolPtr;
use util::check_len;
use value::Value;

pub fn string_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(args[0].try_get_string().is_some())))
}

pub fn make_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(2))?;
    let c = match args.get(1).map(|v| &**v) {
        None => 0 as char,
        Some(Value::Character(c)) => *c,
        _ => {
            return Err(format!(
                "make-string: Invalid initial character: {}.",
                args[1].pretty_print()
            ))
        }
    };
    let l = args[0]
        .try_get_integer()
        .ok_or_else(|| format!("make-string: Invalid length: {}.", args[0].pretty_print()))?;
    let l = l
        .to_usize()
        .ok_or_else(|| format!("make-string: string cannot have negative length: {}.", l))?;
    let s: String = std::iter::repeat(c).take(l).collect();
    Ok(arena.insert(Value::String(RefCell::new(s))))
}

pub fn string_length(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let l = args[0]
        .try_get_string()
        .ok_or_else(|| format!("string-length: not a string: {}", args[0].pretty_print()))?
        .borrow()
        .chars()
        .count();
    Ok(arena.insert(Value::Integer(BigInt::from(l))))
}

pub fn string_set_b(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(3), Some(3))?;
    let mut borrowed_string = args[0]
        .try_get_string()
        .ok_or_else(|| format!("string-set!: not a string: {}", args[0].pretty_print()))?
        .borrow_mut();
    let idx = args[1]
        .try_get_integer()
        .ok_or_else(|| format!("string-set: invalid index: {}", args[1].pretty_print()))?;
    let chr = args[2]
        .try_get_character()
        .ok_or_else(|| format!("string-set: invalid character: {}", args[2].pretty_print()))?;
    let char_idx = idx
        .to_usize()
        .ok_or_else(|| format!("string-ref: invalid index: {}", idx))?;
    let (byte_idx, _) = borrowed_string
        .char_indices()
        .nth(char_idx)
        .ok_or_else(|| format!("string-ref: invalid index: {}", idx))?;
    borrowed_string.replace_range(byte_idx..=byte_idx, &chr.to_string());
    Ok(arena.unspecific)
}

pub fn string_ref(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let borrowed_string = args[0]
        .try_get_string()
        .ok_or_else(|| format!("string-ref: not a string: {}", args[0].pretty_print()))?
        .borrow();
    let idx = args[1]
        .try_get_integer()
        .ok_or_else(|| format!("string-ref: invalid index: {}", args[1].pretty_print()))?;
    let idx = idx
        .to_usize()
        .ok_or_else(|| format!("string-ref: invalid index: {}", idx))?;
    let chr = borrowed_string
        .chars()
        .nth(idx)
        .ok_or_else(|| format!("string-ref: Invalid index: {}.", idx))?;
    Ok(arena.insert(Value::Character(chr)))
}

pub fn string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let values: Result<Vec<_>, String> = args
        .iter()
        .map(|a| {
            a.try_get_character()
                .ok_or_else(|| format!("string: not a char: {}", a.pretty_print()))
        })
        .collect();
    let values = values?;
    Ok(arena.insert(Value::String(RefCell::new(
        values.iter().cloned().collect(),
    ))))
}

fn to_string_vec(args: &[PoolPtr]) -> Result<Vec<Ref<String>>, String> {
    args.iter()
        .map(|v| {
            v.try_get_string()
                .map(|s| s.borrow())
                .ok_or_else(|| format!("not a string: {}", v.pretty_print()))
        })
        .collect::<Result<Vec<_>, String>>()
}

macro_rules! string_cmp {
    ($fun:ident, $w:ident, $e:expr) => {
        pub fn $fun(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
            let strings = to_string_vec(args)?;
            Ok(arena.insert(Value::Boolean(strings.as_slice().windows(2).all(|$w| $e))))
        }
    };
}

string_cmp!(string_equal_p, w, *w[0] == *w[1]);
string_cmp!(string_less_than_p, w, *w[0] < *w[1]);
string_cmp!(string_greater_than_p, w, *w[0] > *w[1]);
string_cmp!(string_less_equal_p, w, *w[0] <= *w[1]);
string_cmp!(string_greater_equal_p, w, *w[0] >= *w[1]);

string_cmp!(
    string_ci_equal_p,
    w,
    w[0].to_ascii_lowercase() == w[1].to_ascii_lowercase()
);
string_cmp!(
    string_ci_less_than_p,
    w,
    w[0].to_ascii_lowercase() < w[1].to_ascii_lowercase()
);
string_cmp!(
    string_ci_greater_than_p,
    w,
    w[0].to_ascii_lowercase() > w[1].to_ascii_lowercase()
);
string_cmp!(
    string_ci_less_equal_p,
    w,
    w[0].to_ascii_lowercase() <= w[1].to_ascii_lowercase()
);
string_cmp!(
    string_ci_greater_equal_p,
    w,
    w[0].to_ascii_lowercase() >= w[1].to_ascii_lowercase()
);
