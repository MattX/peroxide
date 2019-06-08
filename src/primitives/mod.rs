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
//! OK char->integer integer->char
//!
//! OK string?
//! OK make-string string-length string-ref string-set!
//!
//! vector?
//! make-vector vector-length vector-ref vector-set!
//!
//! OK procedure?
//! OK apply
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
use environment::{ActivationFrameInfo, RcEnv};
use primitives::char::*;
use primitives::numeric::*;
use primitives::object::*;
use primitives::pair::*;
pub use primitives::port::Port;
use primitives::port::*;
use primitives::string::*;
use primitives::symbol::*;
pub use primitives::syntactic_closure::SyntacticClosure;
use primitives::syntactic_closure::*;
use primitives::vector::*;
use std::cell::RefCell;
use std::rc::Rc;
use value::Value;

mod char;
mod numeric;
mod object;
mod pair;
mod port;
mod string;
mod symbol;
mod syntactic_closure;
mod vector;

macro_rules! simple_primitive {
    ($name:expr, $implementation:ident) => {
        Primitive {
            name: $name,
            implementation: PrimitiveImplementation::Simple($implementation),
        }
    };
}

static PRIMITIVES: [Primitive; 75] = [
    simple_primitive!("make-syntactic-closure", make_syntactic_closure),
    simple_primitive!("identifier=?", identifier_equal_p),
    simple_primitive!("identifier?", identifier_p),
    simple_primitive!("syntactic-closure?", syntactic_closure_p),
    simple_primitive!(
        "syntactic-closure-environment",
        syntactic_closure_environment
    ),
    simple_primitive!(
        "syntactic-closure-free-variables",
        syntactic_closure_free_variables
    ),
    simple_primitive!("syntactic-closure-expression", syntactic_closure_expression),
    simple_primitive!("gensym", gensym),
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
    simple_primitive!("char-alphabetic?", char_alphabetic_p),
    simple_primitive!("char-numeric?", char_numeric_p),
    simple_primitive!("char-whitespace?", char_whitespace_p),
    simple_primitive!("char-lower-case?", char_lower_case_p),
    simple_primitive!("char-upper-case?", char_upper_case_p),
    simple_primitive!("char-upcase", char_upcase),
    simple_primitive!("char-downcase", char_downcase),
    simple_primitive!("char-upcase-unicode", char_upcase_unicode),
    simple_primitive!("char-downcase-unicode", char_downcase_unicode),
    simple_primitive!("string?", string_p),
    simple_primitive!("make-string", make_string),
    simple_primitive!("string-length", string_length),
    simple_primitive!("string-set!", string_set_b),
    simple_primitive!("string-ref", string_ref),
    simple_primitive!("vector?", vector_p),
    simple_primitive!("make-vector", make_vector),
    simple_primitive!("vector-length", vector_length),
    simple_primitive!("vector-set!", vector_set_b),
    simple_primitive!("vector-ref", vector_ref),
    simple_primitive!("procedure?", procedure_p),
    simple_primitive!("error", error),
    simple_primitive!("port?", port_p),
    simple_primitive!("input-port?", input_port_p),
    simple_primitive!("output-port?", output_port_p),
    simple_primitive!("textual-port?", textual_port_p),
    simple_primitive!("binary-port?", binary_port_p),
    simple_primitive!("close-port", close_port),
    simple_primitive!("port-open?", port_open_p),
    simple_primitive!("open-input-file", open_input_file),
    simple_primitive!("eof-object", eof_object),
    simple_primitive!("eof-object?", eof_object_p),
    simple_primitive!("read-char", read_char),
    simple_primitive!("peek-char", peek_char),
    simple_primitive!("read-line", read_line),
    simple_primitive!("char-ready?", char_ready_p),
    simple_primitive!("read-string", read_string),
    Primitive {
        name: "apply",
        implementation: PrimitiveImplementation::Apply,
    },
    Primitive {
        name: "%call/cc", // The actual call/cc handles dynamic-winds, and is written in Scheme.
        implementation: PrimitiveImplementation::CallCC,
    },
    Primitive {
        name: "raise",
        implementation: PrimitiveImplementation::Raise,
    },
    Primitive {
        name: "abort",
        implementation: PrimitiveImplementation::Abort,
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
    Raise,
    Abort,
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
    let afi = Rc::new(RefCell::new(ActivationFrameInfo {
        parent: None,
        altitude: 0,
        entries: frame.values.len(),
    }));
    for prim in PRIMITIVES.iter() {
        borrowed_env.define(prim.name, &afi, true);
        frame.values.push(arena.insert(Value::Primitive(prim)));
    }
}
