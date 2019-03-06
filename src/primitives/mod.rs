//! **Needed:**
//! eq? eqv?
//!
//! number? complex? real? rational? integer?
//! exact? inexact?
//! = < <= > >=
//!
//! + * - /
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
//! pair?
//! cons car cdr
//! set-car! set-cdr!
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
use environment::Environment;
use primitives::numeric::{add, div, equal, less_than, more_than, mul, sub};
use value::Value;

mod numeric;

static PRIMITIVES: [Primitive; 7] = [
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
        name: "=",
        implementation: equal,
    },
    Primitive {
        name: "<",
        implementation: less_than,
    },
    Primitive {
        name: ">",
        implementation: more_than,
    },
];

#[derive(Clone)]
pub struct Primitive {
    pub name: &'static str,
    pub implementation: fn(&mut Arena, Vec<usize>) -> Result<usize, String>,
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

    fn ne(&self, other: &Primitive) -> bool {
        self.name != other.name
    }
}

pub fn register_primitives(arena: &mut Arena, e: &mut Environment) {
    for p in PRIMITIVES.iter() {
        e.define(p.name, arena.intern(Value::Primitive(p.clone())))
    }
}
