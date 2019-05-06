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
//!
//! TODO: doesn't handle ellipses at all, a few deviations from the standard
//! TODO: not hygienic for now :(

use arena::Arena;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use util::check_len;
use value;
use value::Value;

#[derive(Debug)]
pub struct SyntaxRules {
    pub literals: HashSet<String>,
    pub syntax_rules: Vec<SyntaxRule>,
}

#[derive(Debug)]
pub struct SyntaxRule {
    pub pattern: usize,
    pub template: usize,
}

// TODO: this does not exactly comply with the standard: literals should only match if they
//       have the same binding (or absence thereof).
pub fn parse_transformer_spec(
    arena: &Arena,
    macro_name: &str,
    rest: &[usize],
) -> Result<SyntaxRules, String> {
    check_len(&rest, Some(2), None)?;
    if !is_literal_symbol(arena.get(rest[0]), "syntax-rules") {
        return Err("Invalid transformer spec.".into());
    }

    let literals: Result<HashSet<_>, _> = arena
        .get(rest[1])
        .pair_to_vec(arena)
        .map_err(|e| format!("Syntax error in transformer spec: `{}`", e))?
        .iter()
        .map(|s| match arena.get(*s) {
            Value::Symbol(s) => Ok(s.clone()),
            value => Err(format!(
                "Transformer spec: expected symbols, got `{}`.",
                value.pretty_print(arena)
            )),
        })
        .collect();
    let syntax_rules: Result<Vec<_>, _> = rest[2..]
        .iter()
        .map(|p| parse_syntax_rule(arena, macro_name, *p))
        .collect();
    Ok(SyntaxRules {
        literals: literals?,
        syntax_rules: syntax_rules?,
    })
}

fn parse_syntax_rule(
    arena: &Arena,
    macro_name: &str,
    pattern: usize,
) -> Result<SyntaxRule, String> {
    let def = arena.get(pattern).pair_to_vec(arena)?;
    check_len(&def, Some(2), Some(2))?;

    // TODO: R7RS does not require the first element of a macro to be the macro name.
    let pattern_value = arena.get(def[0]);
    let pattern_without_macro_name = match pattern_value {
        Value::Pair(car, cdr) => {
            let keyword = arena.get(*car.borrow());
            if !is_literal_symbol(keyword, macro_name) {
                return Err(format!(
                    "All syntax rules in macro `{}` must start with `{}`, \
                     but `{}` starts with `{}`",
                    macro_name,
                    macro_name,
                    pattern_value.pretty_print(arena),
                    keyword
                ));
            }
            *cdr.borrow()
        }
        _ => {
            return Err(format!(
                "Invalid syntax rule: expected list, got `{}`.",
                pattern_value.pretty_print(arena)
            ));
        }
    };

    // TODO: verify that no pattern variable appears more than once.

    Ok(SyntaxRule {
        pattern: pattern_without_macro_name,
        template: def[1],
    })
}

fn is_literal_symbol(v: &Value, r: &str) -> bool {
    match v {
        Value::Symbol(s) => s == r,
        _ => false,
    }
}

pub fn expand_macro(arena: &Arena, syntax_rules: SyntaxRules, to: usize) -> Result<usize, String> {
    for syntax_rule in syntax_rules.syntax_rules.iter() {
        if let Some(matched) = match_syntax_rule(arena, &syntax_rules.literals, syntax_rule, to)? {
            return Ok(expand_template(arena, &matched, syntax_rule.template));
        }
    }
    Err("No matching patterns.".into())
}

fn match_syntax_rule(
    arena: &Arena,
    literals: &HashSet<String>,
    syntax_rule: &SyntaxRule,
    to: usize,
) -> Result<Option<HashMap<String, usize>>, String> {
    let mut matched = HashMap::new();

    if do_match_syntax_rule(arena, &mut matched, literals, syntax_rule.pattern, to)? {
        Ok(Some(matched))
    } else {
        Ok(None)
    }
}

fn do_match_syntax_rule(
    arena: &Arena,
    matched_pattern: &mut HashMap<String, usize>,
    literals: &HashSet<String>,
    pattern: usize,
    to: usize,
) -> Result<bool, String> {
    match (arena.get(pattern), arena.get(to)) {
        (Value::Symbol(s), target) => {
            if s == "_" {
                Ok(true)
            } else if literals.contains(s) {
                Ok(is_literal_symbol(target, s))
            } else if matched_pattern.insert(s.clone(), to).is_some() {
                Err(format!("Duplicate symbol in pattern template: `{}`.", s))
            } else {
                Ok(true)
            }
        }
        (Value::Pair(pattern_car, pattern_cdr), Value::Pair(to_car, to_cdr)) => {
            Ok(do_match_syntax_rule(
                arena,
                matched_pattern,
                literals,
                *pattern_car.borrow(),
                *to_car.borrow(),
            )? && do_match_syntax_rule(
                arena,
                matched_pattern,
                literals,
                *pattern_cdr.borrow(),
                *to_cdr.borrow(),
            )?)
        }
        (Value::Vector(pattern_list), Value::Vector(to_list)) =>
        // TODO there's clearly a way to do this without collecting
        {
            Ok(pattern_list
                .iter()
                .zip(to_list.iter())
                .map(|(p, t)| {
                    do_match_syntax_rule(arena, matched_pattern, literals, *p.borrow(), *t.borrow())
                })
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .all(|r| *r))
        }
        _ => Ok(value::equal(arena, pattern, to)),
    }
}

fn expand_template(
    arena: &Arena,
    matched_pattern: &HashMap<String, usize>,
    template: usize,
) -> usize {
    match arena.get(template) {
        Value::Symbol(s) => {
            if let Some(v) = matched_pattern.get(s) {
                *v
            } else {
                template
            }
        }
        Value::Pair(car, cdr) => arena.insert_pair(
            expand_template(arena, matched_pattern, *car.borrow()),
            expand_template(arena, matched_pattern, *cdr.borrow()),
        ),
        Value::Vector(lst) => arena.insert(Value::Vector(
            lst.iter()
                .map(|e| RefCell::new(expand_template(arena, matched_pattern, *e.borrow())))
                .collect(),
        )),
        _ => template,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parse;
    use std::iter::FromIterator;

    fn set_equal(s: &HashSet<String>, v: &[String]) -> bool {
        s.symmetric_difference(&HashSet::from_iter(v.iter().cloned()))
            .count()
            == 0
    }

    #[test]
    fn parse_simple() {
        let arena = Arena::default();
        let rules = parse::read(&arena, "(syntax-rules (abc jkl) ((mac def) (ghi jkl)))").unwrap();
        let rules_vec = arena.get(rules).pair_to_vec(&arena).unwrap();
        let parsed = parse_transformer_spec(&arena, &"mac", &rules_vec).unwrap();
        assert!(set_equal(&parsed.literals, &["abc".into(), "jkl".into()]));
        assert_eq!(parsed.syntax_rules.len(), 1);
        assert!(value::equal(
            &arena,
            parsed.syntax_rules[0].pattern,
            parse::read(&arena, "(def)").unwrap()
        ));
        assert!(value::equal(
            &arena,
            parsed.syntax_rules[0].template,
            parse::read(&arena, "(ghi jkl)").unwrap()
        ));
    }

    #[test]
    fn parse_no_macro_name() {
        let arena = Arena::default();
        let rules = parse::read(&arena, "(syntax-rules () ((not-mac def) (ghi jkl)))").unwrap();
        let rules_vec = arena.get(rules).pair_to_vec(&arena).unwrap();
        let parsed = parse_transformer_spec(&arena, &"mac", &rules_vec);
        assert!(parsed.is_err());
    }
}
