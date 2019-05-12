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

pub fn char_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Character(_) => arena.t,
        _ => arena.f,
    })
}

// TODO char->integer and integer->char are not standards-compliant as far as I understand.

pub fn char_to_integer(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = match arena.get(args[0]) {
        Value::Character(c) => Value::Integer(*c as i64),
        _ => {
            return Err(format!(
                "char->integer: not a char: {}",
                pretty_print(arena, args[0])
            ))
        }
    };
    Ok(arena.insert(res))
}

pub fn integer_to_char(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = match arena.get(args[0]) {
        Value::Integer(i) => Value::Character(*i as u8 as char),
        _ => {
            return Err(format!(
                "integer->char: not an integer: {}",
                pretty_print(arena, args[0])
            ))
        }
    };
    Ok(arena.insert(res))
}
