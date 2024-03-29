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
use environment::{RcAfi, RcEnv};
use heap::{PoolPtr, RootPtr};
use num_traits::ToPrimitive;
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

static PRIMITIVES: [Primitive; 125] = [
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
    simple_primitive!("number?", number_p),
    simple_primitive!("=", equal),
    simple_primitive!("<", less_than),
    simple_primitive!(">", greater_than),
    simple_primitive!("<=", less_than_equal),
    simple_primitive!(">=", greater_than_equal),
    simple_primitive!("+", add),
    simple_primitive!("*", mul),
    simple_primitive!("-", sub),
    simple_primitive!("/", div),
    simple_primitive!("modulo", modulo),
    simple_primitive!("remainder", remainder),
    simple_primitive!("gcd", gcd),
    simple_primitive!("lcm", lcm),
    simple_primitive!("real?", real_p),
    simple_primitive!("rational?", rational_p),
    simple_primitive!("integer?", integer_p),
    simple_primitive!("exact?", exact_p),
    simple_primitive!("inexact", inexact),
    simple_primitive!("exact", exact),
    simple_primitive!("nan?", nan_p),
    simple_primitive!("infinite?", infinite_p),
    simple_primitive!("real-part", real_part),
    simple_primitive!("imag-part", imag_part),
    simple_primitive!("exp", exp),
    simple_primitive!("log", log),
    simple_primitive!("cos", cos),
    simple_primitive!("sin", sin),
    simple_primitive!("tan", tan),
    simple_primitive!("acos", acos),
    simple_primitive!("asin", asin),
    simple_primitive!("%atan", atan),
    simple_primitive!("sqrt", sqrt),
    simple_primitive!("expt", expt),
    simple_primitive!("magnitude", magnitude),
    simple_primitive!("angle", angle),
    simple_primitive!("make-rectangular", make_rectangular),
    simple_primitive!("make-polar", make_polar),
    simple_primitive!("string->number", string_to_number),
    simple_primitive!("number->string", number_to_string),
    simple_primitive!("pair?", pair_p),
    simple_primitive!("cons", cons),
    simple_primitive!("car", car),
    simple_primitive!("cdr", cdr),
    simple_primitive!("set-car!", set_car_b),
    simple_primitive!("set-cdr!", set_cdr_b),
    simple_primitive!("write", write),
    simple_primitive!("display", display),
    simple_primitive!("newline", newline),
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
    simple_primitive!("string", string),
    simple_primitive!("substring", substring),
    simple_primitive!("string->list", string_to_list),
    simple_primitive!("string-append", string_append),
    simple_primitive!("string=?", string_equal_p),
    simple_primitive!("string<?", string_less_than_p),
    simple_primitive!("string>?", string_greater_than_p),
    simple_primitive!("string<=?", string_less_equal_p),
    simple_primitive!("string>=?", string_greater_equal_p),
    simple_primitive!("string-ci=?", string_ci_equal_p),
    simple_primitive!("string-ci<?", string_ci_less_than_p),
    simple_primitive!("string-ci>?", string_ci_greater_than_p),
    simple_primitive!("string-ci<=?", string_ci_less_equal_p),
    simple_primitive!("string-ci>=?", string_ci_greater_equal_p),
    simple_primitive!("open-output-string", open_output_string),
    simple_primitive!("get-output-string", get_output_string),
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
    Primitive {
        name: "eval",
        implementation: PrimitiveImplementation::Eval,
    },
    Primitive {
        name: "current-jiffy",
        implementation: PrimitiveImplementation::CurrentJiffy,
    },
    Primitive {
        name: "load",
        implementation: PrimitiveImplementation::Load,
    },
];

pub struct Primitive {
    pub name: &'static str,
    pub implementation: PrimitiveImplementation,
}

#[derive(Copy, Clone)]
pub enum PrimitiveImplementation {
    Simple(fn(&Arena, &[PoolPtr]) -> Result<PoolPtr, String>),
    Io(fn(&Arena, PoolPtr, PoolPtr, &[PoolPtr]) -> Result<PoolPtr, String>),
    Eval,
    Apply,
    CallCC,
    Raise,
    Abort,
    CurrentJiffy,
    Load,
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

pub fn register_primitives(
    arena: &Arena,
    global_environment: &RcEnv,
    afi: &RcAfi,
    global_frame: &RootPtr,
) {
    let frame = global_frame.pp().long_lived().get_activation_frame();
    for prim in PRIMITIVES.iter() {
        global_environment.borrow_mut().define(prim.name, afi, true);
        let ptr = arena.insert(Value::Primitive(prim));
        frame.borrow_mut().values.push(ptr);
    }
}

pub fn try_get_index(v: PoolPtr) -> Result<usize, String> {
    v.try_get_integer()
        .ok_or_else(|| format!("invalid index: {}", v.pretty_print()))?
        .to_usize()
        .ok_or_else(|| format!("invalid index: {}", v.pretty_print()))
}
