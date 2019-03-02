use std::cell::{Ref, RefMut};
use std::collections::HashMap;
use std::option::Option;

use arena::Arena;
use value::Value;

#[derive(Debug, PartialEq)]
pub struct Environment {
  parent: Option<usize>,
  values: HashMap<String, usize>,
}

impl Environment {
  pub fn new(parent: Option<usize>) -> Environment {
    Environment { parent, values: HashMap::new() }
  }

  pub fn define(&mut self, name: &str, value: usize) {
    self.values.insert(name.to_string(), value);
  }

  // TODO: refactor this and get_from_parent as this mostly looks like a horrific way to coerce
  // the type system into understanding. Ditto for the set methods.
  pub fn get(&self, arena: &Arena, name: &str) -> Option<usize> {
    if let Some(r) = self.values.get(name) {
      Some(*r)
    } else if let Some(p) = self.parent {
      let parent = arena.value_ref(p);
      if let Value::Environment(e) = parent {
        Environment::get_from_parent(e.borrow(), arena, name)
      } else {
        panic!("{:?} (parent of {:?}) is not an environment.", parent, self);
      }
    } else {
      None
    }
  }

  fn get_from_parent(parent: Ref<Environment>, arena: &Arena, name: &str) -> Option<usize> {
    let mut current = parent;

    loop {
      if let Some(r) = current.values.get(name) {
        return Some(*r);
      }

      if let Some(p) = current.parent {
        let parent = arena.value_ref(p);
        if let Value::Environment(e) = parent {
          current = e.borrow();
        } else {
          panic!("{:?} (parent of {:?}) is not an environment.", parent, current);
        }
      } else {
        return None;
      }
    }
  }

  pub fn set(&mut self, arena: &Arena, name: &str, value: usize) -> Result<(), String> {
    if self.values.contains_key(name) {
      self.values.insert(name.to_string(), value);
      Ok(())
    } else if let Some(p) = self.parent {
      let parent = arena.value_ref(p);
      if let Value::Environment(e) = parent {
        Environment::set_in_parent(e.borrow_mut(), arena, name, value)
      } else {
        panic!("{:?} (parent of {:?}) is not an environment.", parent, self);
      }
    } else {
      Err(format!("Key {} not found and environment has no parent", name))
    }
  }

  fn set_in_parent(parent: RefMut<Environment>, arena: &Arena, name: &str, value: usize)
                   -> Result<(), String> {
    let mut current = parent;

    loop {
      if current.values.contains_key(name) {
        current.values.insert(name.to_string(), value);
        return Ok(());
      }

      if let Some(p) = current.parent {
        let parent = arena.value_ref(p);
        if let Value::Environment(e) = parent {
          current = e.borrow_mut();
        } else {
          panic!("{:?} (parent of {:?}) is not an environment.", parent, current);
        }
      } else {
        return Err(format!("Key {} not found and environment has no parent", name))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;

  #[test]
  fn get_empty() {
    let arena = Arena::new();
    let env = Environment::new(None);

    assert_eq!(env.get(&arena, "abc"), None);
  }

  #[test]
  fn define_get() {
    let arena = Arena::new();
    let mut env = Environment::new(None);

    env.define("abc", 22);
    assert_eq!(env.get(&arena, "abc"), Some(22));
  }

  #[test]
  fn define_child() {
    let mut arena = Arena::new();
    let parent_id = arena.intern(Value::Environment(RefCell::new(Environment::new(None))));
    let parent = match arena.value_ref(parent_id) {
      Value::Environment(e) => e,
      _ => panic!("wat")
    };
    let mut child = Environment::new(Some(parent_id));
    child.define("abc", 10);
    assert_eq!(child.get(&arena, "abc"), Some(10));
    assert_eq!(parent.borrow().get(&arena, "abc"), None);
  }

  #[test]
  fn define_parent() {
    let mut arena = Arena::new();
    let parent_id = arena.intern(Value::Environment(RefCell::new(Environment::new(None))));
    let parent = match arena.value_ref(parent_id) {
      Value::Environment(e) => e,
      _ => panic!("wat")
    };
    let child = Environment::new(Some(parent_id));
    parent.borrow_mut().define("abc", 10);
    assert_eq!(child.get(&arena, "abc"), Some(10));
    assert_eq!(parent.borrow().get(&arena, "abc"), Some(10));
  }

  #[test]
  fn define_set_get() {
    let arena = Arena::new();
    let mut env = Environment::new(None);

    env.define("abc", 22);
    env.set(&arena, "abc", 18);
    assert_eq!(env.get(&arena, "abc"), Some(18));
  }
}