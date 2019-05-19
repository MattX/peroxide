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
use std::cell::RefCell;
use util::check_len;
use value::{pretty_print, Value};

pub fn string_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::String(_) => arena.t,
        _ => arena.f,
    })
}

pub fn make_string(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(2))?;
    let c = match args.get(1).map(|v| arena.get(*v)) {
        None => 0 as char,
        Some(Value::Character(c)) => *c,
        _ => {
            return Err(format!(
                "make-string: Invalid initial character: {}.",
                pretty_print(arena, args[1])
            ))
        }
    };
    let l = arena.try_get_integer(args[0]).ok_or(format!(
        "make-string: Invalid length: {}.",
        pretty_print(arena, args[0])
    ))?;
    if l < 0 {
        return Err(format!(
            "make-string: String cannot have negative length: {}.",
            l
        ));
    }
    let s: Vec<char> = std::iter::repeat(c).take(l as usize).collect();
    Ok(arena.insert(Value::String(RefCell::new(s))))
}

pub fn string_length(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let l = arena
        .try_get_string(args[0])
        .ok_or(format!(
            "string-length: Not a string: {}.",
            pretty_print(arena, args[0])
        ))?
        .borrow()
        .len();
    Ok(arena.insert(Value::Integer(l as i64)))
}

pub fn string_set_b(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(3), Some(3))?;
    let mut borrowed_string = arena
        .try_get_string(args[0])
        .ok_or(format!(
            "string-set!: Not a string: {}.",
            pretty_print(arena, args[0])
        ))?
        .borrow_mut();
    let idx = arena.try_get_integer(args[1]).ok_or(format!(
        "string-set: Invalid index: {}.",
        pretty_print(arena, args[1])
    ))?;
    let chr = arena.try_get_character(args[2]).ok_or(format!(
        "string-set: Invalid character: {}.",
        pretty_print(arena, args[2])
    ))?;
    if idx < 0 || idx >= borrowed_string.len() as i64 {
        return Err(format!("string-set!: Invalid index: {}.", idx));
    }
    borrowed_string[idx as usize] = chr;
    Ok(arena.unspecific)
}

pub fn string_ref(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    let borrowed_string = arena
        .try_get_string(args[0])
        .ok_or(format!(
            "string_ref: Not a string: {}.",
            pretty_print(arena, args[0])
        ))?
        .borrow();
    let idx = arena.try_get_integer(args[1]).ok_or(format!(
        "string_ref: Invalid index: {}.",
        pretty_print(arena, args[1])
    ))?;
    if idx < 0 || idx >= borrowed_string.len() as i64 {
        return Err(format!("string_ref: Invalid index: {}.", idx));
    }
    Ok(arena.insert(Value::Character(borrowed_string[idx as usize])))
}