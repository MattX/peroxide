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
use std::collections::HashMap;

use environment::{ActivationFrame, RcEnv};
use gc::Gc;
use primitives::{Port, SyntacticClosure};
use std::ops::Deref;
use value::Value;

pub struct Arena {
    values: Gc<Value>,
    symbol_map: RefCell<HashMap<String, usize>>,
    pub undefined: usize,
    pub unspecific: usize,
    pub eof: usize,
    pub empty_list: usize,
    pub t: usize,
    pub f: usize,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn insert(&self, v: Value) -> usize {
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
                        let pos = self.values.insert(Value::Symbol(s));
                        self.symbol_map.borrow_mut().insert(label, pos);
                        pos
                    }
                }
            }
            _ => self.values.insert(v),
        }
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn get(&self, at: usize) -> &Value {
        self.values.get(at)
    }

    pub fn get_activation_frame(&self, at: usize) -> &RefCell<ActivationFrame> {
        if let Value::ActivationFrame(ref af) = self.get(at) {
            af
        } else {
            panic!("Value is not an activation frame.");
        }
    }

    pub fn try_get_integer(&self, at: usize) -> Option<i64> {
        match self.get(at) {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn try_get_character(&self, at: usize) -> Option<char> {
        match self.get(at) {
            Value::Character(c) => Some(*c),
            _ => None,
        }
    }

    pub fn try_get_string(&self, at: usize) -> Option<&RefCell<String>> {
        match self.get(at) {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_vector(&self, at: usize) -> Option<&RefCell<Vec<usize>>> {
        match self.get(at) {
            Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn try_get_symbol(&self, at: usize) -> Option<&str> {
        match self.get(at) {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn try_get_pair(&self, at: usize) -> Option<(&RefCell<usize>, &RefCell<usize>)> {
        match self.get(at) {
            Value::Pair(car, cdr) => Some((car, cdr)),
            _ => None,
        }
    }

    pub fn try_get_environment(&self, at: usize) -> Option<&RcEnv> {
        match self.get(at) {
            Value::Environment(r) => Some(r),
            _ => None,
        }
    }

    pub fn try_get_syntactic_closure(&self, at: usize) -> Option<&SyntacticClosure> {
        match self.get(at) {
            Value::SyntacticClosure(sc) => Some(sc),
            _ => None,
        }
    }

    pub fn try_get_port(&self, at: usize) -> Option<&Port> {
        match self.get(at) {
            Value::Port(p) => Some(p.deref()),
            _ => None,
        }
    }

    pub fn collect(&mut self, roots: &[usize]) {
        self.values.collect(roots);
    }
}

impl Default for Arena {
    fn default() -> Self {
        let values = Gc::default();
        let undefined = values.insert(Value::Undefined);
        let unspecific = values.insert(Value::Unspecific);
        let eof = values.insert(Value::EofObject);
        let empty_list = values.insert(Value::EmptyList);
        let f = values.insert(Value::Boolean(false));
        let t = values.insert(Value::Boolean(true));
        Arena {
            values,
            symbol_map: RefCell::new(HashMap::new()),
            undefined,
            unspecific,
            eof,
            empty_list,
            f,
            t,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::Value;

    use super::*;

    const BASE_ENTRY: usize = 6;

    #[test]
    fn add_empty() {
        let arena = Arena::default();
        assert_eq!(BASE_ENTRY, arena.insert(Value::Symbol("abc".into())));
    }

    #[test]
    fn get() {
        let arena = Arena::default();
        assert_eq!(BASE_ENTRY, arena.insert(Value::Real(0.1)));
        assert_eq!(Value::Real(0.1), *arena.get(BASE_ENTRY));
    }
}
