use std::cell::RefCell;
use std::collections::HashMap;
use std::option::Option;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
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

    pub fn new_initial(
        parent: Option<Rc<RefCell<Environment>>>,
        bindings: Vec<String>,
    ) -> Environment {
        let mut env = Environment::new(parent);
        for identifier in bindings.iter() {
            env.define(identifier);
        }
        env
    }

    pub fn define(&mut self, name: &str) {
        let value_index = self.values.len();
        self.values.insert(name.to_string(), value_index);
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
