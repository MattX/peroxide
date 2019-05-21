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

// TODO: deduplicate code between here and string.rs

use arena::Arena;
use std::cell::RefCell;
use util::check_len;
use value::{pretty_print, Value};

pub fn vector_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Vector(_) => arena.t,
        _ => arena.f,
    })
}

pub fn make_vector(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(2))?;
    let fill = *args.get(1).unwrap_or(&arena.f);
    let l = arena.try_get_integer(args[0]).ok_or_else(|| {
        format!(
            "make-vector: Invalid length: {}.",
            pretty_print(arena, args[0])
        )
    })?;
    if l < 0 {
        return Err(format!(
            "make-vector: Vector cannot have negative length: {}.",
            l
        ));
    }
    let v: Vec<usize> = std::iter::repeat(fill).take(l as usize).collect();
    Ok(arena.insert(Value::Vector(RefCell::new(v))))
}

pub fn vector_length(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let l = arena
        .try_get_vector(args[0])
        .ok_or_else(|| {
            format!(
                "vector-length: Not a vector: {}.",
                pretty_print(arena, args[0])
            )
        })?
        .borrow()
        .len();
    Ok(arena.insert(Value::Integer(l as i64)))
}

pub fn vector_set_b(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(3), Some(3))?;
    let mut borrowed_vec = arena
        .try_get_vector(args[0])
        .ok_or_else(|| {
            format!(
                "vector-set!: Not a vector: {}.",
                pretty_print(arena, args[0])
            )
        })?
        .borrow_mut();
    let idx = arena.try_get_integer(args[1]).ok_or_else(|| {
        format!(
            "vector-set: Invalid index: {}.",
            pretty_print(arena, args[1])
        )
    })?;
    if idx < 0 || idx >= borrowed_vec.len() as i64 {
        return Err(format!("vector-set!: Invalid index: {}.", idx));
    }
    borrowed_vec[idx as usize] = args[2];
    Ok(arena.unspecific)
}

pub fn vector_ref(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    let borrowed_vec = arena
        .try_get_vector(args[0])
        .ok_or_else(|| {
            format!(
                "vector-ref: Not a vector: {}.",
                pretty_print(arena, args[0])
            )
        })?
        .borrow();
    let idx = arena.try_get_integer(args[1]).ok_or_else(|| {
        format!(
            "vector-ref: Invalid index: {}.",
            pretty_print(arena, args[1])
        )
    })?;
    if idx < 0 || idx >= borrowed_vec.len() as i64 {
        return Err(format!("vector-ref: Invalid index: {}.", idx));
    }
    Ok(borrowed_vec[idx as usize])
}
