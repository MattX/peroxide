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

use std::fmt::Write;

use arena::Arena;
use heap::PoolPtr;
use util::check_len;
use value;
use value::{Value};

pub fn eq_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(args[0] == args[1])))
}

pub fn eqv_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(value::eqv(arena, args[0], args[1]))))
}

pub fn equal_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(value::equal(arena, args[0], args[1]))))
}

pub fn procedure_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Lambda { .. } => arena.t,
        Value::Primitive(_) => arena.t,
        Value::Continuation(_) => arena.t,
        _ => arena.f,
    })
}

pub fn display_to_string(args: &[PoolPtr]) -> String {
    let mut result = String::new();
    for a in args.iter() {
        write!(&mut result, "{}", a.pretty_print()).unwrap();
    }
    result
}

pub fn write(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    print!("{}", display_to_string(args));
    Ok(arena.unspecific)
}

pub fn display(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    for arg in args {
        match arena.get(*arg) {
            Value::String(s) => print!("{}", &s.borrow()),
            Value::Character(c) => print!("{}", c),
            _ => print!("{}", arg.pretty_print()),
        }
    }
    Ok(arena.unspecific)
}

pub fn newline(arena: &Arena, _args: &[PoolPtr]) -> Result<PoolPtr, String> {
    println!();
    Ok(arena.unspecific)
}

pub fn error(_arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    Err(display_to_string(args))
}
