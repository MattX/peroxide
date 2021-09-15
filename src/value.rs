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
use std::rc::Rc;

use arena::Arena;
use compile::CodeBlock;
use environment::{ActivationFrame, RcEnv};
use heap::{PoolPtr, RootPtr};
use lex::CodeRange;
use num_bigint::BigInt;
use num_complex::Complex;
use num_rational::BigRational;
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
    Located(PoolPtr, Box<Locator>),
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
            Value::Symbol(s) => write!(f, "{}", util::escape_symbol(s)),
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
                v.push(*sc.closed_env.borrow());
            }
            Value::Port(p) => p.inventory(v),
            Value::Continuation(c) => c.inventory(v),
            Value::CodeBlock(c) => c.inventory(v),
            _ => (),
        }
    }
}

impl Value {
    pub fn pretty_print(&self) -> String {
        match self {
            Value::Pair(_, _) => self.print_pair(),
            Value::Vector(_) => self.print_vector(),
            Value::SyntacticClosure(SyntacticClosure {
                closed_env,
                free_variables,
                expr,
            }) => format!(
                "#sc[{} {:?} {}]",
                closed_env.borrow().pretty_print(),
                free_variables,
                expr.pretty_print()
            ),
            Value::Continuation(_) => "#<continuation>".to_string(),
            Value::Lambda { code, .. } => match &code.get_code_block().name {
                Some(n) => format!("#<procedure {}>", n),
                None => "#<anonymous procedure>".to_string(),
            },
            Value::Primitive(p) => format!("#<primitive {}>", p.name),
            _ => format!("{}", self),
        }
    }

    fn print_pair(&self) -> String {
        fn _print_pair(p: &Value, s: &mut String) {
            match p {
                Value::Pair(a, b) => {
                    s.push_str(&a.get().pretty_print()[..]);
                    if let Value::EmptyList = &*b.get() {
                        s.push(')');
                    } else {
                        s.push(' ');
                        _print_pair(&*b.get(), s);
                    }
                }
                Value::EmptyList => {
                    s.push(')');
                }
                _ => {
                    s.push_str(&format!(". {})", p)[..]);
                }
            }
        }

        match self {
            Value::Pair(_, _) | Value::EmptyList => {
                let mut s = "(".to_string();
                _print_pair(self, &mut s);
                s
            }
            _ => panic!(
                "print_pair called on a value that is not a pair: {:?}.",
                self
            ),
        }
    }

    fn print_vector(&self) -> String {
        if let Value::Vector(vals) = self {
            let contents = vals
                .borrow()
                .iter()
                .map(|e| e.pretty_print())
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

    pub fn list_to_vec(&self) -> Result<Vec<PoolPtr>, String> {
        let mut p = self;
        let mut result: Vec<PoolPtr> = Vec::new();
        loop {
            match p {
                Value::Pair(car_r, cdr_r) => {
                    result.push(car_r.get());
                    p = cdr_r.get().long_lived();
                }
                Value::EmptyList => break,
                _ => {
                    return Err(format!(
                        "Converting list to vec: {} is not a proper list",
                        self.pretty_print()
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

    pub fn get_activation_frame(&self) -> &RefCell<ActivationFrame> {
        match self {
            Value::ActivationFrame(af) => af,
            _ => panic!("value is not an activation frame"),
        }
    }

    pub fn get_code_block(&self) -> &CodeBlock {
        match self {
            Value::CodeBlock(c) => c,
            _ => panic!("value is not a code block"),
        }
    }

    // TODO make this less verbose with a macro?
    pub fn try_get_integer(&self) -> Option<&BigInt> {
        match self {
            Value::Integer(i) => Some(i),
            _ => None,
        }
    }

    pub fn try_get_character(&self) -> Option<char> {
        match self {
            Value::Character(c) => Some(*c),
            _ => None,
        }
    }

    pub fn try_get_string(&self) -> Option<&RefCell<String>> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_vector(&self) -> Option<&RefCell<Vec<PoolPtr>>> {
        match self {
            Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn try_get_symbol(&self) -> Option<&str> {
        match self {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_pair(&self) -> Option<(&Cell<PoolPtr>, &Cell<PoolPtr>)> {
        match self {
            Value::Pair(car, cdr) => Some((car, cdr)),
            _ => None,
        }
    }

    pub fn try_get_environment(&self) -> Option<&RcEnv> {
        match self {
            Value::Environment(r) => Some(r),
            _ => None,
        }
    }

    pub fn try_get_syntactic_closure(&self) -> Option<&SyntacticClosure> {
        match self {
            Value::SyntacticClosure(sc) => Some(sc),
            _ => None,
        }
    }

    pub fn try_get_port(&self) -> Option<&Port> {
        match self {
            Value::Port(p) => Some(p),
            _ => None,
        }
    }
}

pub fn list_from_vec(arena: &Arena, vals: &[PoolPtr]) -> PoolPtr {
    if vals.is_empty() {
        arena.empty_list
    } else {
        let rest = arena.root(list_from_vec(arena, &vals[1..]));
        arena.insert(Value::Pair(Cell::new(vals[0]), Cell::new(rest.pp())))
    }
}

pub fn eqv(left: PoolPtr, right: PoolPtr) -> bool {
    match (&*left, &*right) {
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
pub fn equal(left: PoolPtr, right: PoolPtr) -> bool {
    match (&*left, &*right) {
        (Value::Pair(left_car, left_cdr), Value::Pair(right_car, right_cdr)) => {
            equal(left_car.get(), right_car.get()) && equal(left_cdr.get(), right_cdr.get())
        }
        (Value::Vector(left_vec), Value::Vector(right_vec)) => left_vec
            .borrow()
            .iter()
            .zip(right_vec.borrow().iter())
            .all(|(l, r)| equal(*l, *r)),
        (Value::String(left_string), Value::String(right_string)) => left_string == right_string,
        _ => eqv(left, right),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    pub file_name: Rc<String>,
    pub range: CodeRange,
}

/// Requires `value` to be rooted?
pub fn strip_locators(arena: &Arena, value: PoolPtr) -> RootPtr {
    match &*value {
        Value::Pair(car, cdr) => {
            let new_car = strip_locators(arena, car.get());
            let new_cdr = strip_locators(arena, cdr.get());
            arena.insert_rooted(Value::Pair(
                Cell::new(new_car.pp()),
                Cell::new(new_cdr.pp()),
            ))
        }
        Value::Vector(rc) => {
            let roots = rc
                .borrow()
                .iter()
                .map(|v| strip_locators(arena, *v))
                .collect::<Vec<_>>();
            arena.insert_rooted(Value::Vector(RefCell::new(
                roots.iter().map(|v| v.pp()).collect(),
            )))
        }
        Value::Located(v, _) => strip_locators(arena, *v),
        _ => arena.root(value),
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
