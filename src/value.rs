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
use std::fmt;
use std::ops::Deref;

use arena::Arena;
use environment::ActivationFrame;
use primitives::Primitive;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Unspecific,
    Real(f64),
    Integer(i64),
    Boolean(bool),
    Character(char),
    Symbol(String),
    String(String),
    EmptyList,
    Pair(RefCell<usize>, RefCell<usize>),
    Vector(Vec<RefCell<usize>>),
    Lambda {
        name: String,
        code: usize,
        environment: usize,
    },
    Primitive(&'static Primitive),
    ActivationFrame(RefCell<ActivationFrame>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Unspecific => write!(f, "#unspecific"),
            Value::Real(r) => write!(f, "{}", r),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::Character('\n') => write!(f, "#\\newline"),
            Value::Character(c) => write!(f, "#\\{}", c),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::String(s) => write!(f, "\"{}\"", s), // TODO escape string
            Value::EmptyList => write!(f, "()"),
            Value::Pair(a, b) => write!(f, "(=>{} . =>{})", a.borrow(), b.borrow()),
            Value::Vector(vals) => {
                let contents = vals
                    .iter()
                    .map(|v| format!("=>{}", v.borrow()))
                    .collect::<Vec<_>>()
                    .join(" ");
                write!(f, "#({})", contents)
            }
            e => write!(f, "{:?}", e),
        }
    }
}

impl Value {
    pub fn pretty_print(&self, arena: &Arena) -> String {
        match self {
            Value::Pair(_, _) => self.print_pair(arena),
            Value::Vector(_) => self.print_vector(arena),
            _ => format!("{}", self),
        }
    }

    fn print_pair(&self, arena: &Arena) -> String {
        fn _print_pair(arena: &Arena, p: &Value, s: &mut String) {
            match p {
                Value::Pair(a, b) => {
                    s.push_str(&arena.value_ref(*a.borrow()).pretty_print(arena)[..]);
                    if let Value::EmptyList = arena.value_ref(*b.borrow().deref()) {
                        s.push_str(")");
                    } else {
                        s.push_str(" ");
                        _print_pair(arena, arena.value_ref(*b.borrow().deref()), s);
                    }
                }
                Value::EmptyList => {
                    s.push_str(")");
                }
                _ => {
                    s.push_str(&format!(". {})", p)[..]);
                }
            }
        }

        match self {
            Value::Pair(_, _) | Value::EmptyList => {
                let mut s = "(".to_string();
                _print_pair(arena, self, &mut s);
                s
            }
            _ => panic!(
                "print_pair called on a value that is not a pair: {:?}.",
                self
            ),
        }
    }

    fn print_vector(&self, arena: &Arena) -> String {
        if let Value::Vector(vals) = self {
            let contents = vals
                .iter()
                .map(|e| arena.value_ref(*e.borrow()).pretty_print(arena))
                .collect::<Vec<_>>()
                .join(" ");
            format!("#({})", contents)
        } else {
            panic!(
                "print_vector called on a value that is not a vector: {:?}.",
                self
            )
        }
    }

    pub fn pair_to_vec(&self, arena: &Arena) -> Result<Vec<usize>, String> {
        let mut p = self;
        let mut result: Vec<usize> = Vec::new();
        loop {
            match p {
                Value::Pair(car_r, cdr_r) => {
                    result.push(*car_r.borrow());
                    p = arena.value_ref(*cdr_r.borrow());
                }
                Value::EmptyList => break,
                _ => {
                    return Err(format!(
                        "Converting list to vec: {} is not a proper list",
                        self.pretty_print(arena)
                    ));
                }
            }
        }
        Ok(result)
    }

    pub fn truthy(&self) -> bool {
        if let Value::Boolean(b) = self {
            *b
        } else {
            true
        }
    }

    pub fn cdr(&self) -> usize {
        if let Value::Pair(_, cdr) = self {
            *cdr.borrow()
        } else {
            panic!("Not a pair: {:?}.", self)
        }
    }

    pub fn list_from_vec(arena: &mut Arena, vals: &[usize]) -> usize {
        if vals.is_empty() {
            arena.empty_list
        } else {
            let rest = Value::list_from_vec(arena, &vals[1..]);
            arena.intern(Value::Pair(RefCell::new(vals[0]), RefCell::new(rest)))
        }
    }
}

/// Structure that holds a function's formal argument list.
/// `(x y z)` will be represented as `Formals { values: [x, y, z], rest: None }`
/// `(x y . z)` will be represented as `Formals { values: [x, y], rest: Some(z) }`
#[derive(Debug, PartialEq, Clone)]
pub struct Formals {
    pub values: Vec<String>,
    pub rest: Option<String>,
}

impl Formals {
    pub fn new(arena: &mut Arena, formals: usize) -> Result<Formals, String> {
        let mut values = Vec::new();
        let mut formal = formals;
        loop {
            match arena.value_ref(formal) {
                Value::Symbol(s) => {
                    return Ok(Formals {
                        values,
                        rest: Some(s.clone()),
                    });
                }
                Value::EmptyList => return Ok(Formals { values, rest: None }),
                Value::Pair(car, cdr) => {
                    if let Value::Symbol(s) = arena.value_ref(*car.borrow()) {
                        values.push(s.clone());
                        formal = *cdr.borrow();
                    } else {
                        return Err(format!(
                            "Malformed formals: {}.",
                            arena.value_ref(formals).pretty_print(arena)
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Malformed formals: {}.",
                        arena.value_ref(formals).pretty_print(arena)
                    ));
                }
            }
        }
    }

    pub fn bind(&self, arena: &mut Arena, args: &[usize]) -> Result<Vec<(String, usize)>, String> {
        if args.len() < self.values.len() {
            return Err("Too few arguments for application.".to_string());
        }
        if args.len() > self.values.len() && self.rest.is_none() {
            return Err("Too many arguments for application.".to_string());
        }

        let mut ans: Vec<_> = self
            .values
            .clone()
            .into_iter()
            .zip(args.to_vec().into_iter())
            .collect();
        if let Some(ref r) = self.rest {
            let num_collected = ans.len();
            ans.push((
                r.clone(),
                Value::list_from_vec(arena, &args[num_collected..]),
            ))
        }
        Ok(ans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_atoms() {
        assert_eq!("3.45", &format!("{}", Value::Real(3.45)));
        assert_eq!("69105", &format!("{}", Value::Integer(69105)));
        assert_eq!("#f", &format!("{}", Value::Boolean(false)));
        assert_eq!("#t", &format!("{}", Value::Boolean(true)));
        assert_eq!("#\\newline", &format!("{}", Value::Character('\n')));
        assert_eq!("#\\x", &format!("{}", Value::Character('x')));
        assert_eq!("abc", &format!("{}", Value::Symbol("abc".to_string())));
        assert_eq!("\"abc\"", &format!("{}", Value::String("abc".to_string())));
    }

    #[test]
    fn format_list() {
        assert_eq!("()", &format!("{}", Value::EmptyList));
        assert_eq!(
            "(=>1 . =>2)",
            &format!("{}", Value::Pair(RefCell::new(1), RefCell::new(2)))
        );
    }

    #[test]
    fn format_vec() {
        assert_eq!("#()", &format!("{}", Value::Vector(vec![])));
        assert_eq!(
            "#(=>1 =>2)",
            &format!("{}", Value::Vector(vec![RefCell::new(1), RefCell::new(2)]))
        );
    }
}
