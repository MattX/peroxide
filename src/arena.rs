use std::borrow::Borrow;

use continuation::Continuation;
use std::cell::RefCell;
/// A global of boxes for all values. This is used to perform garbage collection.
use value::Value;

pub struct Arena {
    values: Vec<ArenaValue>,
    pub unspecific: usize,
    pub empty_list: usize,
    pub tru: usize,
    pub fal: usize,
}

#[derive(Debug, PartialEq)]
enum ArenaValue {
    Absent,
    Present(Box<Value>),
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn intern(&mut self, v: Value) -> usize {
        let space = self.find_space();

        if self.values.len() > 1_000_000 {
            panic!("We're trying to allocate suspicious amounts of memory.");
        }

        match space {
            Some(n) => {
                self.values[n] = ArenaValue::Present(Box::new(v));
                n
            }
            None => {
                self.values.push(ArenaValue::Present(Box::new(v)));
                self.values.len() - 1
            }
        }
    }

    /// Given a position in the arena, returns a reference to the value at that location.
    pub fn value_ref(&self, at: usize) -> &Value {
        match self.values[at] {
            ArenaValue::Absent => panic!("value_ref on absent value."),
            ArenaValue::Present(ref b) => b.borrow(),
        }
    }

    /// Instantiate a new arena
    pub fn new() -> Arena {
        Arena {
            values: vec![
                ArenaValue::Present(Box::new(Value::Unspecific)),
                ArenaValue::Present(Box::new(Value::EmptyList)),
                ArenaValue::Present(Box::new(Value::Boolean(false))),
                ArenaValue::Present(Box::new(Value::Boolean(true))),
            ],
            unspecific: 0,
            empty_list: 1,
            fal: 2,
            tru: 3,
        }
    }

    /// Helper method to intern a continuation in one go
    pub fn intern_continuation(&mut self, c: Continuation) -> usize {
        self.intern(Value::Continuation(RefCell::new(c)))
    }

    pub fn intern_pair(&mut self, car: usize, cdr: usize) -> usize {
        self.intern(Value::Pair(RefCell::new(car), RefCell::new(cdr)))
    }

    /// Returns the address of the first `Absent` value in the arena, or an empty optional if there
    /// is none.
    fn find_space(&self) -> Option<usize> {
        self.values
            .iter()
            .enumerate()
            .filter(|(_index, value)| **value == ArenaValue::Absent)
            .nth(0)
            .map(|(index, _value)| index)
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
    fn add_remove() {
        let mut arena = Arena::new();
        assert_eq!(BASE_ENTRY, arena.intern(Value::EmptyList));
        assert_eq!(BASE_ENTRY + 1, arena.intern(Value::EmptyList));
        assert_eq!(BASE_ENTRY + 2, arena.intern(Value::EmptyList));
        arena.values[BASE_ENTRY + 1] = ArenaValue::Absent;
        assert_eq!(BASE_ENTRY + 1, arena.intern(Value::EmptyList));
    }

    #[test]
    fn get() {
        let mut arena = Arena::new();
        assert_eq!(BASE_ENTRY, arena.intern(Value::Real(0.1)));
        assert_eq!(Value::Real(0.1), *arena.value_ref(BASE_ENTRY));
    }
}
