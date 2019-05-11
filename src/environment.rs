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
//!  * The `Environment` struct holds a mapping of names to (depth, index) coordinates. It's used
//!    at compilation time.
//!  * The `ActivationFrame` struct holds a mapping of (depth, index) coordinates to locations
//!    in the Arena. It's used at runtime.
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

    // The value can be a none to hide a value defined in a parent environment.
    values: HashMap<String, Option<EnvironmentValue>>,
    variable_names: Vec<String>,
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
            values: HashMap::new(),
            variable_names: Vec::new(),
        }
    }

    pub fn new_initial<T: AsRef<str>>(
        parent: Option<Rc<RefCell<Environment>>>,
        bindings: &[T],
    ) -> Environment {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier, true);
        }
        env
    }

    /// Define a new variable. The variable will be added to the topmost environment frame, and
    /// may shadow a variable from a lower frame.
    ///
    /// It is not an error to define a name that already exists in the topmost environment frame.
    /// In this case, a new activation frame location will be allocated to the variable.
    pub fn define(&mut self, name: &str, initialized: bool) -> usize {
        let index = self.variable_names.len();
        self.variable_names.push(name.to_string());
        self.values.insert(
            name.to_string(),
            Some(EnvironmentValue::Variable(Variable {
                altitude: self.altitude,
                index,
                initialized,
            })),
        );
        index
    }

    /// Define a variable if it is not already present. Used for top-level defines.
    pub fn define_if_absent(&mut self, name: &str, initialized: bool) -> usize {
        match self.get(name) {
            Some(EnvironmentValue::Variable(v)) => v.index,
            _ => self.define(name, initialized),
        }
    }

    /// Define a value on the global environment (bottommost frame).
    pub fn define_implicit(&mut self, name: &str) -> usize {
        if let Some(ref e) = self.parent {
            e.borrow_mut().define_implicit(name)
        } else {
            self.define(name, false)
        }
    }

    /// Define a macro in the current environment (topmost frame).
    pub fn define_macro(&mut self, name: &str, value: usize) {
        self.values
            .insert(name.to_string(), Some(EnvironmentValue::Macro(value)));
    }

    pub fn get(&self, name: &str) -> Option<EnvironmentValue> {
        if self.values.contains_key(name) {
            self.values.get(name).and_then(|e| e.clone())
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
            Some(Some(EnvironmentValue::Variable(v))) => v.initialized = true,
            Some(_) => panic!("Tried to mark non-variable as initialized"),
            None => match self.parent {
                Some(ref e) => e.borrow_mut().mark_initialized(name),
                None => panic!(
                    "Tried to mark nonexistent variable `{}` as initialized",
                    name
                ),
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
