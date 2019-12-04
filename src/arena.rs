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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;

use num_bigint::BigInt;

use environment::{ActivationFrame, RcEnv};
use heap;
use heap::RootPtr;
use primitives::{Port, SyntacticClosure};
use value::Value;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ValRef(pub heap::PoolPtr);

impl Deref for ValRef {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct Arena {
    /// Roots held by the arena. This must come before [`values`], or the `Drop` on `RootPtr`
    /// will panic.
    roots: Vec<RootPtr>,
    values: heap::RHeap,
    symbol_map: RefCell<HashMap<String, ValRef>>,
    gensym_counter: Cell<usize>,
    pub undefined: ValRef,
    pub unspecific: ValRef,
    pub eof: ValRef,
    pub empty_list: ValRef,
    pub t: ValRef,
    pub f: ValRef,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn insert(&self, v: Value) -> ValRef {
        match v {
            Value::Undefined => self.undefined,
            Value::Unspecific => self.unspecific,
            Value::EofObject => self.eof,
            Value::EmptyList => self.empty_list,
            Value::Boolean(true) => self.t,
            Value::Boolean(false) => self.f,
            Value::Symbol(s) => {
                let res = self.symbol_map.borrow().get(&s).cloned();
                match res {
                    Some(u) => u,
                    None => {
                        let label = s.clone();
                        let pos = ValRef(self.values.allocate(Value::Symbol(s)));
                        self.symbol_map.borrow_mut().insert(label, pos);
                        pos
                    }
                }
            }
            _ => ValRef(self.values.allocate(v)),
        }
    }

    pub fn root(&self, at: ValRef) -> RootPtr {
        self.values.root(at.0)
    }

    pub fn insert_rooted(&self, v: Value) -> RootPtr {
        self.root(self.insert(v))
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn get<'a>(&'a self, at: ValRef) -> &'a Value {
        unsafe { std::mem::transmute::<&Value, &'a Value>(&*(at.0)) }
    }

    pub fn get_activation_frame(&self, at: ValRef) -> &RefCell<ActivationFrame> {
        if let Value::ActivationFrame(ref af) = self.get(at) {
            af
        } else {
            panic!("Value is not an activation frame.");
        }
    }

    pub fn gensym(&self, base: Option<&str>) -> ValRef {
        let base_str = base.map(|s| format!("{}-", s)).unwrap_or_else(|| "".into());
        loop {
            let candidate = format!("--gs-{}{}", base_str, self.gensym_counter.get());
            self.gensym_counter.set(self.gensym_counter.get() + 1);
            if !self.symbol_map.borrow().contains_key(&candidate) {
                return self.insert(Value::Symbol(candidate));
            }
        }
    }

    pub fn try_get_integer(&self, at: ValRef) -> Option<&BigInt> {
        match self.get(at) {
            Value::Integer(i) => Some(i),
            _ => None,
        }
    }

    pub fn try_get_character(&self, at: ValRef) -> Option<char> {
        match self.get(at) {
            Value::Character(c) => Some(*c),
            _ => None,
        }
    }

    pub fn try_get_string(&self, at: ValRef) -> Option<&RefCell<String>> {
        match self.get(at) {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_vector(&self, at: ValRef) -> Option<&RefCell<Vec<ValRef>>> {
        match self.get(at) {
            Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn try_get_symbol(&self, at: ValRef) -> Option<&str> {
        match self.get(at) {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_pair(&self, at: ValRef) -> Option<(&Cell<ValRef>, &Cell<ValRef>)> {
        match self.get(at) {
            Value::Pair(car, cdr) => Some((car, cdr)),
            _ => None,
        }
    }

    pub fn try_get_environment(&self, at: ValRef) -> Option<&RcEnv> {
        match self.get(at) {
            Value::Environment(r) => Some(r),
            _ => None,
        }
    }

    pub fn try_get_syntactic_closure(&self, at: ValRef) -> Option<&SyntacticClosure> {
        match self.get(at) {
            Value::SyntacticClosure(sc) => Some(sc),
            _ => None,
        }
    }

    pub fn try_get_port(&self, at: ValRef) -> Option<&Port> {
        match self.get(at) {
            Value::Port(p) => Some(p.deref()),
            _ => None,
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        let mut roots = Vec::new();
        let values = heap::RHeap::with_gc_mode(heap::GcMode::Off);

        macro_rules! root {
            ($i: ident, $x: expr) => {
                roots.push(values.allocate_rooted($x));
                let $i = ValRef(roots.last().unwrap().ptr);
            };
        }

        root!(undefined, Value::Undefined);
        root!(unspecific, Value::Unspecific);
        root!(eof, Value::EofObject);
        root!(empty_list, Value::EmptyList);
        root!(f, Value::Boolean(false));
        root!(t, Value::Boolean(true));

        Arena {
            values,
            symbol_map: RefCell::new(HashMap::new()),
            gensym_counter: Cell::new(0),
            undefined,
            unspecific,
            eof,
            empty_list,
            f,
            t,
            roots,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::Value;

    use super::*;

    #[test]
    fn get_symbol() {
        let r = Value::Symbol("abc".into());
        let arena = Arena::default();
        let vr = arena.insert(r.clone());
        assert_eq!(arena.get(vr), &r);
    }

    #[test]
    fn get_number() {
        let arena = Arena::default();
        let vr = arena.insert(Value::Real(0.1));
        assert_eq!(arena.get(vr), &Value::Real(0.1));
    }
}
