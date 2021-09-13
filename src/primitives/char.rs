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

use arena::Arena;
use heap::PoolPtr;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use util::check_len;
use value::Value;

fn get_char_arg(args: &[PoolPtr], prim_name: &str) -> Result<char, String> {
    check_len(args, Some(1), Some(1))?;
    args[0]
        .try_get_character()
        .ok_or_else(|| format!("{}: not a char: {}", prim_name, args[0].pretty_print()))
}

pub fn char_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match &*args[0] {
        Value::Character(_) => arena.t,
        _ => arena.f,
    })
}

pub fn char_to_integer(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char->integer")?;
    let val = Value::Integer(BigInt::from(u32::from(arg)));
    Ok(arena.insert(val))
}

pub fn integer_to_char(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let int = args[0]
        .try_get_integer()
        .ok_or_else(|| format!("integer->char: not an integer: {}", args[0].pretty_print()))?;
    let u32i = int
        .to_u32()
        .ok_or_else(|| format!("integer->char: not a valid char: {}", int))?;
    let res = Value::Character(
        std::char::from_u32(u32i)
            .ok_or_else(|| format!("integer->char: not a valid char: {}", u32i))?,
    );
    Ok(arena.insert(res))
}

// The following methods could be implemented in a library, but they're annoying to implement for
// Unicode values, so we have them as primitives to leverage Rust's Unicode support.

pub fn char_alphabetic_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-alphabetic?")?;
    Ok(arena.insert(Value::Boolean(arg.is_alphabetic())))
}

pub fn char_numeric_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-numeric?")?;
    Ok(arena.insert(Value::Boolean(arg.is_numeric())))
}

pub fn char_whitespace_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-whitespace?")?;
    Ok(arena.insert(Value::Boolean(arg.is_whitespace())))
}

pub fn char_upper_case_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-upper-case?")?;
    Ok(arena.insert(Value::Boolean(arg.is_uppercase())))
}

pub fn char_lower_case_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-lower-case?")?;
    Ok(arena.insert(Value::Boolean(arg.is_lowercase())))
}

// `char::to_uppercase()` and `char::to_lowercase()` use ascii_uppercase and ascii_lowercase,
// because corresponding upper/lower case values can be strings, but the R5RS standard does not
// anticipate this case.

pub fn char_upcase(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-upcase")?;
    Ok(arena.insert(Value::Character(arg.to_ascii_uppercase())))
}

pub fn char_downcase(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-downcase")?;
    Ok(arena.insert(Value::Character(arg.to_ascii_lowercase())))
}

pub fn char_upcase_unicode(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-upcase-unicode")?;
    Ok(arena.insert(Value::String(RefCell::new(arg.to_uppercase().to_string()))))
}

pub fn char_downcase_unicode(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    let arg = get_char_arg(args, "char-downcase-unicode")?;
    Ok(arena.insert(Value::String(RefCell::new(arg.to_lowercase().to_string()))))
}
