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

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;

use value::Value;

pub struct Arena {
    values: Vec<Value>,
    symbol_map: HashMap<String, usize>,
    pub unspecific: usize,
    pub empty_list: usize,
    pub t: usize,
    pub f: usize,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn intern(&mut self, v: Value) -> usize {
        match v {
            Value::Unspecific => self.unspecific,
            Value::EmptyList => self.empty_list,
            Value::Boolean(true) => self.t,
            Value::Boolean(false) => self.f,
            Value::Symbol(s) => {
                let res = self.symbol_map.get(&s).cloned();
                match res {
                    Some(u) => u,
                    None => {
                        let label = s.clone();
                        let pos = self.do_intern(Value::Symbol(s));
                        self.symbol_map.insert(label, pos);
                        pos
                    }
                }
            }
            _ => self.do_intern(v),
        }
    }

    /// Actually does the thing described above
    fn do_intern(&mut self, v: Value) -> usize {
        self.values.push(v);
        self.values.len() - 1
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn value_ref(&self, at: usize) -> &Value {
        match self.values.get(at) {
            None => panic!("Tried to access invalid arena location {}", at),
            Some(v) => v.borrow(),
        }
    }

    pub fn swap_out(&mut self, at: usize) -> Value {
        std::mem::replace(&mut self.values[at], Value::Unspecific)
    }

    pub fn swap_in(&mut self, at: usize, v: Value) {
        if let Value::Unspecific = self.values[at] {
            self.values[at] = v;
        } else {
            panic!("Swapping in non-unspecific value");
        }
    }

    pub fn intern_pair(&mut self, car: usize, cdr: usize) -> usize {
        self.intern(Value::Pair(RefCell::new(car), RefCell::new(cdr)))
    }
}

impl Default for Arena {
    fn default() -> Self {
        Arena {
            values: vec![
                Value::Unspecific,
                Value::EmptyList,
                Value::Boolean(false),
                Value::Boolean(true),
            ],
            symbol_map: HashMap::new(),
            unspecific: 0,
            empty_list: 1,
            f: 2,
            t: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::Value;

    use super::*;

    const BASE_ENTRY: usize = 4;

    #[test]
    fn add_empty() {
        let mut arena = Arena::default();
        assert_eq!(BASE_ENTRY, arena.intern(Value::Symbol("abc".into())));
    }

    #[test]
    fn get() {
        let mut arena = Arena::default();
        assert_eq!(BASE_ENTRY, arena.intern(Value::Real(0.1)));
        assert_eq!(Value::Real(0.1), *arena.value_ref(BASE_ENTRY));
    }
}
