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

// TODO: deduplicate code between here and string.rs

use std::cell::RefCell;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use arena::Arena;
use heap::PoolPtr;
use util::check_len;
use value::Value;

pub fn vector_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(match arena.get(args[0]) {
        Value::Vector(_) => arena.t,
        _ => arena.f,
    })
}

pub fn make_vector(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(2))?;
    let fill = *args.get(1).unwrap_or(&arena.f);
    let l = arena
        .try_get_integer(args[0])
        .ok_or_else(|| format!("make-vector: Invalid length: {}.", args[0].pretty_print()))?;
    let l = l
        .to_usize()
        .ok_or_else(|| format!("make-vector: string cannot have negative length: {}.", l))?;
    let v: Vec<PoolPtr> = std::iter::repeat(fill).take(l as usize).collect();
    Ok(arena.insert(Value::Vector(RefCell::new(v))))
}

pub fn vector_length(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let l = arena
        .try_get_vector(args[0])
        .ok_or_else(|| format!("vector-length: Not a vector: {}.", args[0].pretty_print()))?
        .borrow()
        .len();
    Ok(arena.insert(Value::Integer(BigInt::from(l))))
}

pub fn vector_set_b(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(3), Some(3))?;
    let mut borrowed_vec = arena
        .try_get_vector(args[0])
        .ok_or_else(|| format!("vector-set!: Not a vector: {}.", args[0].pretty_print()))?
        .borrow_mut();
    let idx = arena
        .try_get_integer(args[1])
        .ok_or_else(|| format!("vector-set: Invalid index: {}.", args[1].pretty_print()))?;
    let idx = idx
        .to_usize()
        .ok_or_else(|| format!("vector-set!: Invalid index: {}.", idx))?;
    if idx >= borrowed_vec.len() {
        return Err(format!("vector-set!: Invalid index: {}.", idx));
    }
    borrowed_vec[idx] = args[2];
    Ok(arena.unspecific)
}

pub fn vector_ref(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let borrowed_vec = arena
        .try_get_vector(args[0])
        .ok_or_else(|| format!("vector-ref: Not a vector: {}.", args[0].pretty_print()))?
        .borrow();
    let idx = arena
        .try_get_integer(args[1])
        .ok_or_else(|| format!("vector-ref: Invalid index: {}.", args[1].pretty_print()))?;
    let idx = idx
        .to_usize()
        .ok_or_else(|| format!("vector-set!: Invalid index: {}.", idx))?;
    borrowed_vec
        .get(idx)
        .copied()
        .ok_or_else(|| format!("vector-set!: Invalid index: {}.", idx))
}
