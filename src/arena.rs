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

use gc::Gc;
use value::Value;

pub struct Arena {
    values: Gc<Value>,
    symbol_map: RefCell<HashMap<String, usize>>,
    pub unspecific: usize,
    pub empty_list: usize,
    pub t: usize,
    pub f: usize,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn intern(&self, v: Value) -> usize {
        match v {
            Value::Unspecific => self.unspecific,
            Value::EmptyList => self.empty_list,
            Value::Boolean(true) => self.t,
            Value::Boolean(false) => self.f,
            Value::Symbol(s) => {
                let res = self.symbol_map.borrow().get(&s).cloned();
                match res {
                    Some(u) => u,
                    None => {
                        let label = s.clone();
                        let pos = self.do_intern(Value::Symbol(s));
                        self.symbol_map.borrow_mut().insert(label, pos);
                        pos
                    }
                }
            }
            _ => self.do_intern(v),
        }
    }

    /// Actually does the thing described above
    fn do_intern(&self, v: Value) -> usize {
        self.values.insert(v)
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn value_ref(&self, at: usize) -> &Value {
        self.values.get(at)
    }

    pub fn intern_pair(&mut self, car: usize, cdr: usize) -> usize {
        self.intern(Value::Pair(RefCell::new(car), RefCell::new(cdr)))
    }

    pub fn collect(&mut self, roots: &[usize]) {
        self.values.collect(roots);
    }
}

impl Default for Arena {
    fn default() -> Self {
        let values = Gc::default();
        let unspecific = values.insert(Value::Unspecific);
        let empty_list = values.insert(Value::EmptyList);
        let f = values.insert(Value::Boolean(false));
        let t = values.insert(Value::Boolean(true));
        Arena {
            values,
            symbol_map: RefCell::new(HashMap::new()),
            unspecific,
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
