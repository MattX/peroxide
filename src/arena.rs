use std::borrow::Borrow;

use continuation::Continuation;
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

    /// Instantiate a new arena
    pub fn new() -> Arena {
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

    /// Helper method to intern a continuation in one go
    pub fn intern_continuation(&mut self, c: Continuation) -> usize {
        self.intern(Value::Continuation(c))
    }

    pub fn intern_pair(&mut self, car: usize, cdr: usize) -> usize {
        self.intern(Value::Pair(RefCell::new(car), RefCell::new(cdr)))
    }
}

#[cfg(test)]
mod tests {
    use value::Value;

    use super::*;

    const BASE_ENTRY: usize = 4;

    #[test]
    fn add_empty() {
        let mut arena = Arena::new();
        assert_eq!(BASE_ENTRY, arena.intern(Value::EmptyList));
    }

    #[test]
    fn get() {
        let mut arena = Arena::new();
        assert_eq!(BASE_ENTRY, arena.intern(Value::Real(0.1)));
        assert_eq!(Value::Real(0.1), *arena.value_ref(BASE_ENTRY));
    }
}
