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

pub fn char_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
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
    let l = match arena.get(args[0]) {
        Value::Integer(i) if *i >= 0 => *i as usize,
        _ => {
            return Err(format!(
                "make-string: Invalid length: {}.",
                pretty_print(arena, args[0])
            ))
        }
    };
    let s: Vec<char> = std::iter::repeat(c).take(l).collect();
    Ok(arena.insert(Value::String(RefCell::new(s))))
}

pub fn string_length(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let l = match arena.get(args[0]) {
        Value::String(s) => s.borrow().len() as i64,
        _ => {
            return Err(format!(
                "string-length: Not a string: {}.",
                pretty_print(arena, args[0])
            ))
        }
    };
    Ok(arena.insert(Value::Integer(l)))
}

pub fn string_set_b(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(3), Some(3))?;
    let borrowed_string = match arena.get(args[0]) {
        Value::String(s) => s.borrow(),
        _ => {
            return Err(format!(
                "string-set!: Not a string: {}.",
                pretty_print(arena, args[0])
            ))
        }
    };
    // TODO finish
    Ok(arena.unspecific)
}
