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

use std::cell::RefCell;

use arena::Arena;
use heap::PoolPtr;
use util::check_len;
use value::{pretty_print, Value};

pub fn symbol_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Symbol(_) => arena.t,
        _ => arena.f,
    })
}

pub fn symbol_to_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Symbol(s) => Ok(arena.insert(Value::String(RefCell::new(s.clone())))),
        _ => Err(format!(
            "symbol->string: not a symbol: {}",
            pretty_print(arena, args[0])
        )),
    }
}

pub fn string_to_symbol(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::String(s) => Ok(arena.insert(Value::Symbol(s.borrow().clone()))),
        _ => Err(format!(
            "string->symbol: not a string: {}",
            pretty_print(arena, args[0])
        )),
    }
}
