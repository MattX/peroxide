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

use std::cell::{Cell, RefCell};
use std::fmt;

use num_bigint::BigInt;
use num_complex::Complex;
use num_rational::BigRational;

use arena::Arena;
use compile::CodeBlock;
use environment::{ActivationFrame, RcEnv};
use heap::PoolPtr;
use primitives::{Port, Primitive, SyntacticClosure};
use vm::Continuation;
use {heap, util};

// TODO box some of these, values are currently 56 bytes long oh no
// TODO remove PartialEq and Clone. Clone should only be used in the numeric primitives library.
//      PartialEq is used in a number of unit / integ tests, but could be replaced with equal_p
//      from this file.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Unspecific,
    EofObject,
    EmptyList,
    Real(f64),
    Integer(BigInt),
    Rational(Box<BigRational>),
    ComplexReal(Complex<f64>),
    ComplexInteger(Box<Complex<BigInt>>),
    ComplexRational(Box<Complex<BigRational>>),
    Boolean(bool),
    Character(char),
    Symbol(String),
    String(RefCell<String>),
    Pair(Cell<PoolPtr>, Cell<PoolPtr>),
    ByteVector(RefCell<Vec<u8>>),
    Vector(RefCell<Vec<PoolPtr>>),
    Lambda { code: PoolPtr, frame: PoolPtr },
    Port(Box<Port>),
    Primitive(&'static Primitive),
    ActivationFrame(RefCell<ActivationFrame>),
    Environment(RcEnv),
    SyntacticClosure(SyntacticClosure),
    Continuation(Continuation),
    CodeBlock(Box<CodeBlock>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "#undefined"),
            Value::Unspecific => write!(f, "#unspecific"),
            Value::EofObject => write!(f, "#eof-object"),
            Value::EmptyList => write!(f, "()"),
            Value::Real(r) => write!(f, "{}", r),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Rational(r) => write!(f, "{}", r),
            Value::ComplexReal(c) => write!(f, "{}", c),
            Value::ComplexInteger(c) => write!(f, "{}", c),
            Value::ComplexRational(c) => write!(f, "{}", c),
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::Character('\n') => write!(f, "#\\newline"),
            Value::Character(c) => write!(f, "#\\{}", util::escape_char(*c)),
            Value::Symbol(s) => write!(f, "{}", util::escape_symbol(&s)),
            Value::String(s) => write!(f, "\"{}\"", util::escape_string(&s.borrow())),
            Value::Pair(a, b) => write!(f, "({} . {})", &*a.get(), &*b.get()),
            Value::ByteVector(bv) => {
                let contents = bv
                    .borrow()
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                write!(f, "#u8({})", contents)
            }
            Value::Vector(values) => {
                let contents = values
                    .borrow()
                    .iter()
                    .map(|v| format!("=>{:?}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                write!(f, "#({})", contents)
            }
            Value::Environment(rce) => write!(f, "{:?}", rce.borrow()),
            e => write!(f, "{:?}", e),
        }
    }
}

impl heap::Inventory for Value {
    fn inventory(&self, v: &mut heap::PtrVec) {
        match self {
            Value::Pair(car, cdr) => {
                v.push(car.get());
                v.push(cdr.get());
            }
            Value::Vector(vals) => {
                for val in vals.borrow().iter() {
                    v.push(*val);
                }
            }
            Value::Lambda { code, frame } => {
                v.push(*code);
                v.push(*frame);
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
            Value::SyntacticClosure(sc) => {
                v.push(sc.expr);
                v.push(sc.closed_env.borrow().clone());
            }
            Value::Port(p) => p.inventory(v),
            Value::Continuation(c) => c.inventory(v),
            Value::CodeBlock(c) => c.inventory(v),
            _ => (),
        }
    }
}

impl Value {
    pub fn pretty_print(&self, arena: &Arena) -> String {
        match self {
            Value::Pair(_, _) => self.print_pair(arena),
            Value::Vector(_) => self.print_vector(arena),
            Value::SyntacticClosure(SyntacticClosure {
                closed_env,
                free_variables,
                expr,
            }) => format!(
                "#sc[{} {:?} {}]",
                pretty_print(arena, *closed_env.borrow()),
                free_variables,
                arena.get(*expr).pretty_print(arena)
            ),
            Value::Continuation(_) => "#<continuation>".to_string(),
            _ => format!("{}", self),
        }
    }

    fn print_pair(&self, arena: &Arena) -> String {
        fn _print_pair(arena: &Arena, p: &Value, s: &mut String) {
            match p {
                Value::Pair(a, b) => {
                    s.push_str(&arena.get(a.get()).pretty_print(arena)[..]);
                    if let Value::EmptyList = arena.get(b.get()) {
                        s.push_str(")");
                    } else {
                        s.push_str(" ");
                        _print_pair(arena, arena.get(b.get()), s);
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

    pub fn pair_to_vec(&self, arena: &Arena) -> Result<Vec<PoolPtr>, String> {
        let mut p = self;
        let mut result: Vec<PoolPtr> = Vec::new();
        loop {
            match p {
                Value::Pair(car_r, cdr_r) => {
                    result.push(car_r.get());
                    p = arena.get(cdr_r.get());
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
pub fn vec_from_list(arena: &Arena, val: PoolPtr) -> Result<Vec<PoolPtr>, String> {
    arena.get(val).pair_to_vec(arena)
}

pub fn list_from_vec(arena: &Arena, vals: &[PoolPtr]) -> PoolPtr {
    if vals.is_empty() {
        arena.empty_list
    } else {
        let rest = arena.root(list_from_vec(arena, &vals[1..]));
        arena.insert(Value::Pair(Cell::new(vals[0]), Cell::new(rest.pp())))
    }
}

// TODO phase out inline version
pub fn pretty_print(arena: &Arena, at: PoolPtr) -> String {
    arena.get(at).pretty_print(arena)
}

pub fn eqv(arena: &Arena, left: PoolPtr, right: PoolPtr) -> bool {
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
pub fn equal(arena: &Arena, left: PoolPtr, right: PoolPtr) -> bool {
    match (arena.get(left), arena.get(right)) {
        (Value::Pair(left_car, left_cdr), Value::Pair(right_car, right_cdr)) => {
            equal(arena, left_car.get(), right_car.get())
                && equal(arena, left_cdr.get(), right_cdr.get())
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
        assert_eq!("69105", &format!("{}", Value::Integer(69105.into())));
        assert_eq!("#f", &format!("{}", Value::Boolean(false)));
        assert_eq!("#t", &format!("{}", Value::Boolean(true)));
        assert_eq!("#\\newline", &format!("{}", Value::Character('\n')));
        assert_eq!("#\\x", &format!("{}", Value::Character('x')));
        assert_eq!("abc", &format!("{}", Value::Symbol("abc".to_string())));
        assert_eq!(
            "\"abc\"",
            &format!("{}", Value::String(RefCell::new("abc".to_string())))
        );
    }
}
