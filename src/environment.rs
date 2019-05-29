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
use std::fmt::{Debug, Error, Formatter};
use std::option::Option;
use std::rc::Rc;
use value::Value;

pub struct Environment {
    parent: Option<Rc<RefCell<Environment>>>,

    // The value can be a none to hide a value defined in a parent environment.
    values: HashMap<String, Option<EnvironmentValue>>,

    /// Map of (altitude, index) to variable name.
    variable_names: HashMap<(usize, usize), String>,
}

impl PartialEq for Environment {
    fn eq(&self, _other: &Self) -> bool {
        panic!("Comparing environments")
    }
}

impl Debug for Environment {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if let Some(ref p) = self.parent {
            write!(f, "{:?} ← {:?}", p.borrow(), self.values.keys())
        } else {
            write!(f, "<toplevel>")
        }
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

#[derive(Clone)]
pub struct Macro {
    pub lambda: usize,
    pub definition_environment: RcEnv,
}

impl std::fmt::Debug for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        // hide the environment field to avoid environment -> macro -> environment reference loops
        write!(f, "Macro{{Lambda={}}}", self.lambda)
    }
}

impl Environment {
    pub fn new(parent: Option<Rc<RefCell<Environment>>>) -> Self {
        Environment {
            parent,
            values: HashMap::new(),
            variable_names: HashMap::new(),
        }
    }

    pub fn new_initial<T: AsRef<str>>(
        parent: Option<Rc<RefCell<Environment>>>,
        af_info: &RcAfi,
        bindings: &[T],
    ) -> Self {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier.as_ref(), af_info, true);
        }
        env
    }

    /// Define a new variable. The variable will be added to the topmost environment frame, and
    /// may shadow a variable from a lower frame.
    ///
    /// The passed ActivationFrameInfo will be updated.
    ///
    /// It is not an error to define a name that already exists in the topmost environment frame.
    /// In this case, a new activation frame location will be allocated to the variable.
    pub fn define(&mut self, name: &str, af_info: &RcAfi, initialized: bool) -> usize {
        let index = af_info.borrow().entries;
        af_info.borrow_mut().entries += 1;
        self.define_explicit(name, af_info.borrow().altitude, index, initialized)
    }

    /// Define a value on the global environment (bottommost frame).
    pub fn define_toplevel(&mut self, name: &str, af_info: &RcAfi) -> usize {
        if let Some(ref e) = self.parent {
            e.borrow_mut().define_toplevel(name, af_info)
        } else {
            let toplevel_afi = get_toplevel_afi(af_info);
            self.define(name, &toplevel_afi, false)
        }
    }

    /// Define a variable if it is not already present. Used for top-level defines.
    ///
    /// Returns the index of the variable in either case.
    pub fn define_if_absent(&mut self, name: &str, af_info: &RcAfi, initialized: bool) -> usize {
        match self.get(name) {
            Some(EnvironmentValue::Variable(v)) => v.index,
            _ => self.define(name, af_info, initialized),
        }
    }

    /// Define a variable pointing to a specific index in the frame.
    pub fn define_explicit(
        &mut self,
        name: &str,
        altitude: usize,
        index: usize,
        initialized: bool,
    ) -> usize {
        self.variable_names
            .insert((altitude, index), name.to_string());
        self.values.insert(
            name.to_string(),
            Some(EnvironmentValue::Variable(Variable {
                altitude,
                index,
                initialized,
            })),
        );
        index
    }

    /// Define a macro in the current environment (topmost frame).
    ///
    /// It is legal to call [define_macro] with a name that is already used by a macro. In this
    /// case, the macro will be replaced.
    ///
    /// TODO: definition environment should be a weak ref to avoid cycles?
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

    pub fn get_name(&self, altitude: usize, index: usize) -> String {
        if let Some(s) = self.variable_names.get(&(altitude, index)) {
            s.clone()
        } else if let Some(ref e) = self.parent {
            e.borrow().get_name(altitude, index)
        } else {
            format!("unnamed variable {}/{}", altitude, index)
        }
    }

    pub fn parent(&self) -> Option<&RcEnv> {
        (&self.parent).as_ref()
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

pub fn filter(closed_env: &RcEnv, free_env: &RcEnv, free_vars: &[String]) -> Result<RcEnv, String> {
    // TODO: there are some conditions under which syntactic closures may point to nonexistent
    //       locations, because they have been popped off. We should take care of that somehow.

    let mut filtered = Environment::new(Some(closed_env.clone()));
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

    pub fn depth(&self, arena: &Arena) -> usize {
        if let Some(p) = self.get_parent(arena) {
            p.borrow().depth(arena) + 1
        } else {
            0
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

#[derive(Debug)]
pub struct ActivationFrameInfo {
    pub parent: Option<Rc<RefCell<ActivationFrameInfo>>>,
    pub altitude: usize,
    pub entries: usize,
}

pub type RcAfi = Rc<RefCell<ActivationFrameInfo>>;

impl ActivationFrameInfo {
    pub fn add_entry(&mut self) -> usize {
        let entry_index = self.entries;
        self.entries += 1;
        entry_index
    }
}

impl Default for ActivationFrameInfo {
    fn default() -> Self {
        ActivationFrameInfo {
            parent: None,
            altitude: 0,
            entries: 0,
        }
    }
}

pub fn extend_af_info(af_info: &RcAfi) -> RcAfi {
    let new_af_info = ActivationFrameInfo {
        parent: Some(af_info.clone()),
        altitude: af_info.borrow().altitude + 1,
        entries: 0,
    };
    Rc::new(RefCell::new(new_af_info))
}

pub fn get_toplevel_afi(af_info: &RcAfi) -> RcAfi {
    let borrowed_afi = af_info.borrow();
    if let Some(ref p) = borrowed_afi.parent.clone() {
        get_toplevel_afi(p)
    } else {
        af_info.clone()
    }
}
