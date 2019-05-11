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
use value;
use value::Value;

pub fn eq_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(args[0] == args[1])))
}

pub fn eqv_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(value::eqv(arena, args[0], args[1]))))
}

pub fn equal_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.insert(Value::Boolean(value::equal(arena, args[0], args[1]))))
}

// TODO rename this to write and create an actual display method.
pub fn display(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    for a in args.iter() {
        print!("{} ", arena.get(*a).pretty_print(arena));
    }
    println!();
    Ok(arena.unspecific)
}
