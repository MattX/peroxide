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

//! Naming conventions in Rust: replace `?` with `_p`, `!` with `_b`, `->` with `_to_`.
//!
//! ### Needed
//! OK~ eq? eqv? equal?
//!
//! number? complex? real? rational? integer?
//! exact? inexact?
//! OK = < <= > >=
//!
//! OK + * - /
//! quotient remainder modulo
//! numerator denominator
//! floor ceiling truncate round
//! exp log sin cos tan asin acos atan atan2
//! sqrt expt
//!
//! make-rectangular make-polar real-part imag-part magnitude angle
//! exact->inexact inexact->exact
//! number->string string->number
//!
//! OK pair?
//! OK cons car cdr
//! OK set-car! set-cdr!
//!
//! OK symbol?
//! OK symbol->string
//! OK string->symbol
//!
//! OK char?
//! char=? char<? char<=? char>? char>=?
//! OK char->integer integer->char
//!
//! string?
//! make-string string-length string-ref string-set!
//!
//! vector?
//! make-vector vector-length vector-ref vector-set!
//!
//! procedure?
//! apply
//!
//! call-with-current-continuation
//! values call-with-values dynamic-wind ~> library or not?
//!
//! eval scheme-report-environment null-environment
//!
//! input-port? output-port?
//! current-input-port current-output-port
//! open-input-file open-output-file
//! close-input-port close-output-port
//!
//! read-char peek-char eof-object? char-ready? write-char
//!
//! load

use std::fmt::{Debug, Error, Formatter};

use arena::Arena;
use environment::RcEnv;
use primitives::char::*;
use primitives::extensions::*;
use primitives::numeric::*;
use primitives::object::*;
use primitives::pair::*;
use primitives::symbol::*;
use value::Value;

mod char;
mod extensions;
mod numeric;
mod object;
mod pair;
mod symbol;

macro_rules! simple_primitive {
    ($name:expr, $implementation:ident) => {
        Primitive {
            name: $name,
            implementation: PrimitiveImplementation::Simple($implementation),
        }
    };
}

static PRIMITIVES: [Primitive; 29] = [
    simple_primitive!("make-syntactic-closure", make_syntactic_closure),
    simple_primitive!("eq?", eq_p),
    simple_primitive!("eqv?", eqv_p),
    simple_primitive!("equal?", equal_p),
    simple_primitive!("=", equal),
    simple_primitive!("<", less_than),
    simple_primitive!(">", greater_than),
    simple_primitive!("<=", less_than_equal),
    simple_primitive!(">=", greater_than_equal),
    simple_primitive!("+", add),
    simple_primitive!("*", mul),
    simple_primitive!("-", sub),
    simple_primitive!("/", div),
    simple_primitive!("integer?", integer_p),
    simple_primitive!("real?", real_p),
    simple_primitive!("pair?", pair_p),
    simple_primitive!("cons", cons),
    simple_primitive!("car", car),
    simple_primitive!("cdr", cdr),
    simple_primitive!("set-car!", set_car_b),
    simple_primitive!("set-cdr!", set_cdr_b),
    simple_primitive!("display", display),
    simple_primitive!("symbol?", symbol_p),
    simple_primitive!("symbol->string", symbol_to_string),
    simple_primitive!("string->symbol", string_to_symbol),
    simple_primitive!("char?", char_p),
    simple_primitive!("char->integer", char_to_integer),
    simple_primitive!("integer->char", integer_to_char),
    Primitive {
        name: "apply",
        implementation: PrimitiveImplementation::Apply,
    },
];

pub struct Primitive {
    pub name: &'static str,
    pub implementation: PrimitiveImplementation,
}

pub enum PrimitiveImplementation {
    Simple(fn(&Arena, &[usize]) -> Result<usize, String>),
    Eval,
    Apply,
    CallCC,
}

impl Debug for Primitive {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "primitive {}", self.name)
    }
}

impl PartialEq for Primitive {
    fn eq(&self, other: &Primitive) -> bool {
        self.name == other.name
    }
}

pub fn register_primitives(arena: &Arena, global_environment: &RcEnv, global_frame: usize) {
    let mut borrowed_env = global_environment.borrow_mut();
    let mut frame = arena.get_activation_frame(global_frame).borrow_mut();
    for prim in PRIMITIVES.iter() {
        borrowed_env.define(prim.name, true);
        frame.values.push(arena.insert(Value::Primitive(prim)));
    }
}
