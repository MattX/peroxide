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

//! Environments are split into two parts for performance reasons:
//!  * The `Environment` struct holds a mapping of names to (depth, index) coordinates
//!  * The `ActivationFrame` struct holds a mapping of (depth, index) coordinates to locations
//!    in the Arena.

use arena::Arena;
use std::cell::RefCell;
use std::collections::HashMap;
use std::option::Option;
use std::rc::Rc;
use value::Value;

#[derive(Debug, PartialEq)]
pub struct Environment {
    parent: Option<Rc<RefCell<Environment>>>,
    values: HashMap<String, usize>,
}

impl Environment {
    pub fn new(parent: Option<Rc<RefCell<Environment>>>) -> Environment {
        Environment {
            parent,
            values: HashMap::new(),
        }
    }

    pub fn new_initial(parent: Option<Rc<RefCell<Environment>>>, bindings: &[&str]) -> Environment {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier);
        }
        env
    }

    pub fn define(&mut self, name: &str) -> usize {
        let value_index = self.values.len();
        self.values.insert(name.to_string(), value_index);
        self.values.len() - 1
    }

    pub fn get(&self, name: &str) -> Option<(usize, usize)> {
        if self.values.contains_key(name) {
            self.values.get(name).map(|i| (0, *i))
        } else if let Some(ref e) = self.parent {
            e.borrow().get(name).map(|(d, i)| (d + 1, i))
        } else {
            None
        }
    }
}

pub type RcEnv = Rc<RefCell<Environment>>;

#[derive(Debug, PartialEq, Clone)]
pub struct ActivationFrame {
    pub parent: Option<usize>,
    pub values: Vec<usize>,
}

impl ActivationFrame {
    pub fn get_parent<'a>(&self, arena: &'a Arena) -> Option<&'a RefCell<Self>> {
        self.parent.map(|p| {
            if let Value::ActivationFrame(af) = arena.value_ref(p) {
                af
            } else {
                panic!("Parent of ActivationFrame is {:?}", arena.value_ref(p))
            }
        })
    }

    pub fn get(&self, arena: &Arena, depth: usize, index: usize) -> usize {
        if depth == 0 {
            self.values[index]
        } else if let Some(p) = self.get_parent(arena) {
            p.borrow().get(arena, depth - 1, index)
        } else {
            panic!("Accessing depth with no parent.")
        }
    }

    pub fn set(&mut self, arena: &Arena, depth: usize, index: usize, value: usize) {
        if depth == 0 {
            self.values[index] = value;
        } else if let Some(p) = self.get_parent(arena) {
            p.borrow_mut().set(arena, depth - 1, index, value);
        } else {
            panic!("Accessing depth with no parent.");
        }
    }
}

pub struct CombinedEnv {
    pub env: RcEnv,
    pub frame: usize,
}
