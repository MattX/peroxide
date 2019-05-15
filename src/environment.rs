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
use util::same_object;
use value::Value;

#[derive(Debug)]
pub struct Environment {
    parent: Option<Rc<RefCell<Environment>>>,
    altitude: usize,

    // The value can be a none to hide a value defined in a parent environment.
    values: HashMap<String, Option<EnvironmentValue>>,
    variable_names: HashMap<(usize, usize), String>,
    variable_count: usize,
}

// Gruik
impl PartialEq for Environment {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub enum EnvironmentValue {
    Macro(Macro),
    Variable(Variable),
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub altitude: usize,
    pub index: usize,
    pub initialized: bool,
}

#[derive(Debug, Clone)]
pub struct Macro {
    pub lambda: usize,
    pub definition_environment: RcEnv,
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
            variable_names: HashMap::new(),
            variable_count: 0,
        }
    }

    pub fn new_initial<T: AsRef<str>>(
        parent: Option<Rc<RefCell<Environment>>>,
        bindings: &[T],
    ) -> Environment {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier.as_ref(), true);
        }
        env
    }

    pub fn new_syntactic(parent: &Rc<RefCell<Environment>>) -> Environment {
        Environment {
            parent: Some(parent.clone()),
            altitude: parent.borrow().altitude,
            values: HashMap::new(),
            variable_names: HashMap::new(),
            variable_count: 0,
        }
    }

    /// Define a new variable. The variable will be added to the topmost environment frame, and
    /// may shadow a variable from a lower frame.
    ///
    /// It is not an error to define a name that already exists in the topmost environment frame.
    /// In this case, a new activation frame location will be allocated to the variable.
    pub fn define(&mut self, name: &str, initialized: bool) -> usize {
        let index = self.variable_count;
        self.variable_count += 1;
        self.variable_names
            .insert((self.altitude, index), name.to_string());
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
    ///
    /// It is legal to call [define_macro] with a name that is already used by a macro. In this
    /// case, the macro will be replaced.
    pub fn define_macro(&mut self, name: &str, lambda: usize, definition_environment: RcEnv) {
        self.values.insert(
            name.to_string(),
            Some(EnvironmentValue::Macro(Macro {
                lambda,
                definition_environment,
            })),
        );
    }

    pub fn get(&self, name: &str) -> Option<EnvironmentValue> {
        if self.values.contains_key(name) {
            self.values.get(name).and_then(Clone::clone)
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

    pub fn altitude(&self) -> usize {
        self.altitude
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

fn is_parent_of(parent: &RcEnv, kid: &RcEnv) -> bool {
    if same_object::<Environment>(&parent.borrow(), &kid.borrow()) {
        return true;
    }
    match &kid.borrow().parent {
        None => false,
        Some(p) => is_parent_of(parent, p),
    }
}

pub fn filter(closed_env: &RcEnv, free_env: &RcEnv, free_vars: &[String]) -> Result<RcEnv, String> {
    // Free bindings should always be to an environment down the chain.
    if !is_parent_of(&closed_env, &free_env) {
        return Err("Syntactic closure used outside of definition environment.".into());
    }

    let mut filtered = Environment::new_syntactic(closed_env);
    for free_var in free_vars.iter() {
        let var = free_env.borrow().get(free_var);
        filtered.values.insert(free_var.clone(), var.clone());
        if let Some(EnvironmentValue::Variable(v)) = var {
            filtered
                .variable_names
                .insert((v.altitude, v.index), free_var.clone());
        }
    }

    Ok(Rc::new(RefCell::new(filtered)))
}

// TODO make these fields private and have proper accessors
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

    /// Guarantees that subsequent gets to `index`, or any lower index, on the toplevel
    /// environment, will be in bounds.
    ///
    /// Can only be called on the toplevel environment itself.
    pub fn ensure_index(&mut self, arena: &Arena, index: usize) {
        if self.parent.is_some() {
            panic!("ActivationFrame::ensure_size() called on non-root activation frame.");
        }
        if index >= self.values.len() {
            self.values.resize(index + 1, arena.undefined)
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
