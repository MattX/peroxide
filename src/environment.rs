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
//!
//! By convention, depth refers to the distance to the current environment (so 0 is the most
//! local environment), and altitude refers to the distance to the global environment (so 0 is
//! the global environment).

use arena::Arena;
use std::cell::RefCell;
use std::collections::HashMap;
use std::option::Option;
use std::rc::Rc;
use value::Value;

#[derive(Debug)]
pub struct Environment {
    parent: Option<Rc<RefCell<Environment>>>,
    altitude: usize,
    variable_count: usize,
    values: HashMap<String, EnvironmentValue>,
}

#[derive(Debug, Clone)]
pub enum EnvironmentValue {
    Macro(usize),
    Variable(Variable),
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub altitude: usize,
    pub index: usize,
    pub initialized: bool,
}

impl Environment {
    pub fn new(parent: Option<Rc<RefCell<Environment>>>) -> Environment {
        let altitude = parent
            .as_ref()
            .map(|p| p.borrow().altitude + 1)
            .unwrap_or(0);
        Environment {
            parent,
            altitude,
            variable_count: 0,
            values: HashMap::new(),
        }
    }

    pub fn new_initial(parent: Option<Rc<RefCell<Environment>>>, bindings: &[&str]) -> Environment {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier, true);
        }
        env
    }

    pub fn define(&mut self, name: &str, initialized: bool) -> usize {
        let index = self.variable_count;
        self.variable_count += 1;
        self.values.insert(
            name.to_string(),
            EnvironmentValue::Variable(Variable {
                altitude: self.altitude,
                index,
                initialized,
            }),
        );
        index
    }

    pub fn define_toplevel(&mut self, name: &str, initialized: bool) -> usize {
        if let Some(ref e) = self.parent {
            e.borrow_mut().define_toplevel(name, initialized)
        } else {
            self.define(name, initialized)
        }
    }

    pub fn define_macro(&mut self, name: &str, value: usize) {
        self.values
            .insert(name.to_string(), EnvironmentValue::Macro(value));
    }

    pub fn get(&self, name: &str) -> Option<EnvironmentValue> {
        if self.values.contains_key(name) {
            self.values.get(name).cloned()
        } else if let Some(ref e) = self.parent {
            e.borrow().get(name)
        } else {
            None
        }
    }

    /// Returns the depth of a variable of the given altitude, assuming this environment is
    /// current.
    pub fn depth(&self, altitude: usize) -> usize {
        self.altitude - altitude
    }

    pub fn mark_initialized(&mut self, name: &str) {
        match self.values.get_mut(name) {
            Some(EnvironmentValue::Variable(v)) => {
                if v.initialized {
                    panic!("Tried to mark already-initialized value as initialized");
                } else {
                    v.initialized = false;
                }
            }
            Some(_) => panic!("Tried to mark non-variable as initialized"),
            None => match self.parent {
                Some(ref e) => e.borrow_mut().mark_initialized(name),
                None => panic!("Tried to mark nonexistent variable {} as initialized", name),
            },
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
            if let Value::ActivationFrame(af) = arena.get(p) {
                af
            } else {
                panic!("Parent of ActivationFrame is {:?}", arena.get(p))
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
