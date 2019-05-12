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
use util::{char_vec_to_str, check_len, str_to_char_vec};
use value::{pretty_print, Value};

pub fn symbol_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Symbol(_) => arena.t,
        _ => arena.f,
    })
}

pub fn symbol_to_string(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Symbol(s) => {
            let string = str_to_char_vec(s);
            Ok(arena.insert(Value::String(RefCell::new(string))))
        }
        _ => Err(format!(
            "symbol->string: not a symbol: {}",
            pretty_print(arena, args[0])
        )),
    }
}

pub fn string_to_symbol(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::String(s) => {
            let symbol = char_vec_to_str(&s.borrow());
            Ok(arena.insert(Value::Symbol(symbol)))
        }
        _ => Err(format!(
            "string->symbol: not a string: {}",
            pretty_print(arena, args[0])
        )),
    }
}
