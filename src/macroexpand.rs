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

//! Macro-related stuff. Named macroexpand because macro is a reserved word in Rust

use arena::Arena;
use util::check_len;
use value::Value;

pub struct SyntaxRules {
    pub keywords: Vec<String>,
    pub patterns: Vec<Pattern>,
}

pub struct Pattern {
    pub reference: usize,
    pub replacement: usize,
}

pub fn parse_transformer_spec(arena: &mut Arena, rest: &[usize]) -> Result<SyntaxRules, String> {
    check_len(&rest, Some(2), None)?;
    let syntax_rules_ok = match arena.get(rest[0]) {
        Value::Symbol(s) => s == "syntax-rules",
        _ => false,
    };
    if !syntax_rules_ok {
        return Err("Invalid transformer spec.".into());
    }
    let keywords: Result<Vec<_>, _> = arena
        .get(rest[1])
        .pair_to_vec(arena)
        .map_err(|e| format!("Syntax error in transformer spec: {}", e))?
        .iter()
        .map(|s| {
            let v = arena.get(*s);
            match v {
                Value::Symbol(s) => Ok(s.clone()),
                _ => Err(format!(
                    "Transformer spec: expected symbols, got {}.",
                    v.pretty_print(arena)
                )),
            }
        })
        .collect();
    let patterns: Result<Vec<_>, _> = rest[2..].iter().map(|p| parse_pattern(arena, *p)).collect();
    Ok(SyntaxRules {
        keywords: keywords?,
        patterns: patterns?,
    })
}

pub fn parse_pattern(arena: &mut Arena, pattern: usize) -> Result<Pattern, String> {
    let def = arena.get(pattern).pair_to_vec(arena)?;
    check_len(&def, Some(2), Some(2))?;
    Err("".into())
}
