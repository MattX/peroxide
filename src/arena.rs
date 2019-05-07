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

use gc::Gc;
use value::Value;

pub struct Arena {
    values: Gc<Value>,
    symbol_map: RefCell<HashMap<String, usize>>,
    gensym_counter: Cell<usize>,
    pub unspecific: usize,
    pub empty_list: usize,
    pub t: usize,
    pub f: usize,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn insert(&self, v: Value) -> usize {
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
                        let pos = self.values.insert(Value::Symbol(s));
                        self.symbol_map.borrow_mut().insert(label, pos);
                        pos
                    }
                }
            }
            _ => self.values.insert(v),
        }
    }

    /// Generates a new symbol that's unique unless the programmer decides to name their own
    /// identifiers `__gensym_xyz` for some reason.
    pub fn gensym(&self, name: Option<&str>) -> usize {
        let underscore_name = name.map(|n| format!("_{}", n)).unwrap_or_else(|| "".into());
        self.gensym_counter.set(self.gensym_counter.get() + 1);
        self.insert(Value::Symbol(format!(
            "__gensym{}_{}",
            underscore_name,
            self.gensym_counter.get()
        )))
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn get(&self, at: usize) -> &Value {
        self.values.get(at)
    }

    pub fn insert_pair(&self, car: usize, cdr: usize) -> usize {
        self.insert(Value::Pair(RefCell::new(car), RefCell::new(cdr)))
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
            gensym_counter: Cell::new(0),
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
