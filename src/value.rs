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
use environment::{ActivationFrame, RcEnv};
use gc;
use primitives::Primitive;
use util::char_vec_to_str;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Undefined,
    Unspecific,
    Real(f64),
    Integer(i64),
    Boolean(bool),
    Character(char),
    Symbol(String),
    String(RefCell<Vec<char>>),
    EmptyList,
    Pair(RefCell<usize>, RefCell<usize>),
    Vector(RefCell<Vec<usize>>),
    Lambda {
        code: usize,
        environment: usize,
    },
    Primitive(&'static Primitive),
    ActivationFrame(RefCell<ActivationFrame>),
    Environment(RcEnv),
    SyntacticClosure {
        closed_env: usize,
        free_variables: Vec<String>,
        expr: usize,
    },
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "#undefined"),
            Value::Unspecific => write!(f, "#unspecific"),
            Value::Real(r) => write!(f, "{}", r),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::Character('\n') => write!(f, "#\\newline"),
            Value::Character(c) => write!(f, "#\\{}", c),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::String(s) => {
                write!(f, "\"{}\"", char_vec_to_str(&s.borrow())) // TODO escape string
            }
            Value::EmptyList => write!(f, "()"),
            Value::Pair(a, b) => write!(f, "(=>{} . =>{})", a.borrow(), b.borrow()),
            Value::Vector(values) => {
                let contents = values
                    .borrow()
                    .iter()
                    .map(|v| format!("=>{}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                write!(f, "#({})", contents)
            }
            e => write!(f, "{:?}", e),
        }
    }
}

impl gc::Inventory for Value {
    fn inventory(&self, v: &mut gc::PushOnlyVec<usize>) {
        match self {
            Value::Pair(car, cdr) => {
                v.push(*car.borrow());
                v.push(*cdr.borrow());
            }
            Value::Vector(vals) => {
                for val in vals.borrow().iter() {
                    v.push(*val);
                }
            }
            Value::Lambda { environment, .. } => {
                v.push(*environment);
            }
            Value::ActivationFrame(af) => {
                let f = af.borrow();
                if let Some(p) = f.parent {
                    v.push(p)
                };
                for val in f.values.iter() {
                    v.push(*val)
                }
            }
            _ => (),
        }
    }
}

impl Value {
    pub fn pretty_print(&self, arena: &Arena) -> String {
        match self {
            Value::Pair(_, _) => self.print_pair(arena),
            Value::Vector(_) => self.print_vector(arena),
            Value::SyntacticClosure {
                closed_env,
                free_variables,
                expr,
            } => format!(
                "#syntactic-closure[{} {:?} {}]",
                closed_env,
                free_variables,
                arena.get(*expr).pretty_print(arena)
            ),
            _ => format!("{}", self),
        }
    }

    fn print_pair(&self, arena: &Arena) -> String {
        fn _print_pair(arena: &Arena, p: &Value, s: &mut String) {
            match p {
                Value::Pair(a, b) => {
                    s.push_str(&arena.get(*a.borrow()).pretty_print(arena)[..]);
                    if let Value::EmptyList = arena.get(*b.borrow().deref()) {
                        s.push_str(")");
                    } else {
                        s.push_str(" ");
                        _print_pair(arena, arena.get(*b.borrow().deref()), s);
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
                .borrow()
                .iter()
                .map(|e| arena.get(*e).pretty_print(arena))
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
                    p = arena.get(*cdr_r.borrow());
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
}

// TODO phase out inline version
pub fn vec_from_list(arena: &Arena, val: usize) -> Result<Vec<usize>, String> {
    arena.get(val).pair_to_vec(arena)
}

pub fn list_from_vec(arena: &Arena, vals: &[usize]) -> usize {
    if vals.is_empty() {
        arena.empty_list
    } else {
        let rest = list_from_vec(arena, &vals[1..]);
        arena.insert(Value::Pair(RefCell::new(vals[0]), RefCell::new(rest)))
    }
}

// TODO phase out inline version
pub fn pretty_print(arena: &Arena, at: usize) -> String {
    arena.get(at).pretty_print(arena)
}

pub fn eqv(arena: &Arena, left: usize, right: usize) -> bool {
    match (arena.get(left), arena.get(right)) {
        // This comparison is in the same order as the R5RS one for ease of
        // verification.
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Real(a), Value::Real(b)) => (a - b).abs() < std::f64::EPSILON,
        (Value::Character(a), Value::Character(b)) => a == b,
        (Value::EmptyList, Value::EmptyList) => true,
        (Value::Pair(_, _), Value::Pair(_, _)) => left == right,
        (Value::Vector(_), Value::Vector(_)) => left == right,
        (Value::String(_), Value::String(_)) => left == right,
        (Value::Lambda { .. }, Value::Lambda { .. }) => left == right,
        _ => false,
    }
}

//TODO should not loop on recursive data (R7RS)
pub fn equal(arena: &Arena, left: usize, right: usize) -> bool {
    match (arena.get(left), arena.get(right)) {
        (Value::Pair(left_car, left_cdr), Value::Pair(right_car, right_cdr)) => {
            equal(arena, *left_car.borrow(), *right_car.borrow())
                && equal(arena, *left_cdr.borrow(), *right_cdr.borrow())
        }
        (Value::Vector(left_vec), Value::Vector(right_vec)) => left_vec
            .borrow()
            .iter()
            .zip(right_vec.borrow().iter())
            .all(|(l, r)| equal(arena, *l, *r)),
        (Value::String(left_string), Value::String(right_string)) => left_string == right_string,
        _ => eqv(arena, left, right),
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
        assert_eq!(
            "\"abc\"",
            &format!("{}", Value::String(RefCell::new(vec!['a', 'b', 'c'])))
        );
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
        assert_eq!("#()", &format!("{}", Value::Vector(RefCell::new(vec![]))));
        assert_eq!(
            "#(=>1 =>2)",
            &format!("{}", Value::Vector(RefCell::new(vec![1, 2])))
        );
    }
}
