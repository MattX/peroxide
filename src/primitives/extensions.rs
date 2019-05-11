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
use value::{pretty_print, vec_from_list, Value};

pub fn make_syntactic_closure(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(3), Some(3))?;
    let free_variables = vec_from_list(arena, args[1])?
        .iter()
        .map(|fv| match arena.get(*fv) {
            Value::Symbol(s) => Ok(s.clone()),
            _ => Err(format!(
                "make-syntactic-closure: not a symbol: {}",
                pretty_print(arena, *fv)
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let environment = match arena.get(args[0]) {
        Value::Environment(_) => Ok(args[0]),
        _ => Err(format!(
            "make-syntactic-closure: not an environment: {}",
            pretty_print(arena, args[0])
        )),
    }?;
    Ok(arena.insert(Value::SyntacticClosure {
        environment,
        free_variables,
        expr: args[2],
    }))
}
