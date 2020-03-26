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

use std::cell::Cell;

use arena::Arena;
use heap::PoolPtr;
use util::check_len;
use value::Value;

pub fn pair_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let ans = match arena.get(args[0]) {
        Value::Pair(_, _) => true,
        _ => false,
    };
    Ok(arena.insert(Value::Boolean(ans)))
}

pub fn cons(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Pair(Cell::new(args[0]), Cell::new(args[1]))))
}

pub fn car(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Pair(car, _) => Ok(car.get()),
        _ => Err(format!(
            "called car on a non-pair: {}",
            args[0].pretty_print()
        )),
    }
}

pub fn cdr(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.get(args[0]) {
        Value::Pair(_, cdr) => Ok(cdr.get()),
        _ => Err(format!(
            "called cdr on a non-pair: {}",
            args[0].pretty_print()
        )),
    }
}

pub fn set_car_b(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.get(args[0]) {
        Value::Pair(car, _) => Ok(car.replace(args[1])),
        _ => Err(format!(
            "called set-car! on a non-pair: {}",
            args[0].pretty_print()
        )),
    }
}

pub fn set_cdr_b(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.get(args[0]) {
        Value::Pair(_, cdr) => Ok(cdr.replace(args[1])),
        _ => Err(format!(
            "called set-cdr! on a non-pair: {}",
            args[0].pretty_print()
        )),
    }
}

// Code for a loop-compatible length function.
/*
enum ListType {
    Invalid,
    Empty,
    Some(usize),
}

fn next(arena: &Arena, pair: usize) -> ListType {
    match arena.get(pair[0]) {
        Value::EmptyList => ListType::Empty,
        Value::Pair(car, cdr) => ListType::Some(cdr.borrow().clone()),
        _ => ListType::Invalid
    }
}

fn next_twice(arena: &Arena, pair: usize) -> ListType {
    match next(arena, pair) {
        ListType::Some(s) => next(arena, s),
        e => e
    }
}

pub fn length(arena: &Arena, args: &[ValRef]) -> Result<ValRef, String> {
    check_len(args, Some(1), Some(1))?;

    let mut slow = args[0];
    let mut fast = slow;
    let mut len = 0usize;

    loop {
        match arena.get(slow) {
            Value::EmptyList => Ok(arena.insert(Value::Integer(len.into()))),
            Value::Pair(car, cdr) => {
                loop {}
            }
        }
    }
}
*/
