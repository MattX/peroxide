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
use value::Value;

pub fn pair_p(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let ans = match arena.get(args[0]) {
        Value::Pair(_, _) => true,
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(ans)))
}

pub fn cons(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Pair(RefCell::new(args[0]), RefCell::new(args[1]))))
}

pub fn car(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Pair(car, _) => Ok(*car.borrow()),
        _ => Err(format!(
            "Called car on a non-pair: {}",
            arena.get(args[0]).pretty_print(arena)
        )),
    }
}

pub fn cdr(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Pair(_, cdr) => Ok(*cdr.borrow()),
        _ => Err(format!(
            "Called cdr on a non-pair: {}",
            arena.get(args[0]).pretty_print(arena)
        )),
    }
}

pub fn set_car_b(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.get(args[0]) {
        Value::Pair(car, _) => Ok(car.replace(args[1])),
        _ => Err(format!(
            "Called set-car! on a non-pair: {}",
            arena.get(args[0]).pretty_print(arena)
        )),
    }
}

pub fn set_cdr_b(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.get(args[0]) {
        Value::Pair(_, cdr) => Ok(cdr.replace(args[1])),
        _ => Err(format!(
            "Called set-cdr! on a non-pair: {}",
            arena.get(args[0]).pretty_print(arena)
        )),
    }
}
