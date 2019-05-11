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
//! symbol?
//! symbol->string
//! string->symbol
//!
//! char?
//! char=? char<? char<=? char>? char>=?
//! char->integer integer->char
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
use environment::CombinedEnv;
use primitives::extensions::*;
use primitives::numeric::*;
use primitives::object::*;
use primitives::pair::*;
use std::cell::RefCell;
use value::Value;

mod extensions;
mod numeric;
mod object;
mod pair;

static PRIMITIVES: [Primitive; 20] = [
    Primitive {
        name: "eq?",
        implementation: eq_p,
    },
    Primitive {
        name: "eqv?",
        implementation: eqv_p,
    },
    Primitive {
        name: "equal?",
        implementation: equal_p,
    },
    Primitive {
        name: "=",
        implementation: equal,
    },
    Primitive {
        name: "<",
        implementation: less_than,
    },
    Primitive {
        name: ">",
        implementation: greater_than,
    },
    Primitive {
        name: "<=",
        implementation: less_than_equal,
    },
    Primitive {
        name: ">=",
        implementation: greater_than_equal,
    },
    Primitive {
        name: "+",
        implementation: add,
    },
    Primitive {
        name: "*",
        implementation: mul,
    },
    Primitive {
        name: "-",
        implementation: sub,
    },
    Primitive {
        name: "/",
        implementation: div,
    },
    Primitive {
        name: "pair?",
        implementation: pair_p,
    },
    Primitive {
        name: "cons",
        implementation: cons,
    },
    Primitive {
        name: "car",
        implementation: car,
    },
    Primitive {
        name: "cdr",
        implementation: cdr,
    },
    Primitive {
        name: "set-car!",
        implementation: set_car_b,
    },
    Primitive {
        name: "set-cdr!",
        implementation: set_cdr_b,
    },
    Primitive {
        name: "display",
        implementation: display,
    },
    Primitive {
        name: "make-syntactic-closure",
        implementation: make_syntactic_closure,
    },
];

#[derive(Clone)]
pub struct Primitive {
    pub name: &'static str,
    pub implementation: fn(&Arena, &[usize]) -> Result<usize, String>,
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

pub fn register_primitives(arena: &Arena, e: &mut CombinedEnv) {
    // Intern all the primitives before getting the frame to avoid a mut/non-mut alias to the
    // arena.

    let primitive_indices: Vec<_> = PRIMITIVES
        .iter()
        .map(|p| arena.insert(Value::Primitive(p)))
        .collect();
    let mut borrowed_env = RefCell::borrow_mut(&e.env);
    let mut frame = if let Value::ActivationFrame(af) = arena.get(e.frame) {
        af.borrow_mut()
    } else {
        panic!("Frame is not actually an activation frame");
    };
    for (prim, interned) in PRIMITIVES.iter().zip(primitive_indices.into_iter()) {
        borrowed_env.define(prim.name, true);
        frame.values.push(interned);
    }
}
