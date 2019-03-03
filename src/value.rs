use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;

use arena::Arena;
use environment::Environment;
use continuation::Continuation;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
  Real(f64),
  Integer(i64),
  Boolean(bool),
  Character(char),
  Symbol(String),
  String(String),
  EmptyList,
  Pair(RefCell<usize>, RefCell<usize>),
  Vector(Vec<RefCell<usize>>),
  Environment(RefCell<Environment>),
  Continuation(RefCell<Continuation>),
  // We'll add some stuff here later
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Value::Real(r) => write!(f, "{}", r),
      Value::Integer(i) => write!(f, "{}", i),
      Value::Boolean(true) => write!(f, "#t"),
      Value::Boolean(false) => write!(f, "#f"),
      Value::Character('\n') => write!(f, "#\\newline"),
      Value::Character(c) => write!(f, "#\\{}", c),
      Value::Symbol(s) => write!(f, "{}", s),
      Value::String(s) => write!(f, "\"{}\"", s), // TODO escape string
      Value::EmptyList => write!(f, "()"),
      Value::Pair(a, b) => write!(f, "(=>{} . =>{})", a.borrow(), b.borrow()),
      Value::Vector(vals) => {
        let contents = vals.iter()
            .map(|v| format!("=>{}", v.borrow()))
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "#({})", contents)
      }
      e => write!(f, "{:?}", e)
    }
  }
}

impl Value {
  pub fn pretty_print(&self, arena: &Arena) -> String {
    match self {
      Value::Pair(_, _) => self.print_pair(arena),
      Value::Vector(_) => self.print_vector(arena),
      _ => format!("{}", self)
    }
  }

  fn print_pair(&self, arena: &Arena) -> String {
    fn _print_pair(arena: &Arena, p: &Value, s: &mut String) {
      match p {
        Value::Pair(a, b) => {
          s.push_str(&arena.value_ref(*a.borrow()).pretty_print(arena)[..]);
          if let Value::EmptyList = arena.value_ref(*b.borrow().deref()) {
            s.push_str(")");
          } else {
            s.push_str(&format!(" "));
            _print_pair(arena, arena.value_ref(*b.borrow().deref()), s);
          }
        }
        Value::EmptyList => {
          s.push_str(")");
        }
        _ => {
          s.push_str(&format!(". {})", p)[..]);
        }
      }
    }

    match self {
      Value::Pair(_, _) | Value::EmptyList => {
        let mut s = "(".to_string();
        _print_pair(arena, self, &mut s);
        s
      }
      _ => panic!("print_pair called on a value that is not a pair: {:?}.", self)
    }
  }

  fn print_vector(&self, arena: &Arena) -> String {
    if let Value::Vector(vals) = self {
      let contents = vals.iter()
          .map(|e| format!("{}", arena.value_ref(*e.borrow()).pretty_print(arena)))
          .collect::<Vec<String>>()
          .join(" ");
      format!("#({})", contents)
    } else {
      panic!("print_vector called on a value that is not a vector: {:?}.", self)
    }
  }

  pub fn pair_to_vec(&self, arena: &Arena) -> Result<Vec<usize>, String> {
    let mut p = self;
    let mut result: Vec<usize> = Vec::new();
    loop {
      match p {
        Value::Pair(car_r, cdr_r) => {
          result.push(*car_r.borrow());
          p = arena.value_ref(*cdr_r.borrow());
        }
        Value::EmptyList => break,
        _ => return Err(format!("Converting list to vec: {} is not a proper list.",
                                self.pretty_print(arena)))
      }
    }
    Ok(result)
  }

  pub fn truthy(&self) -> bool {
    if let Value::Boolean(b) = self {
      *b
    } else {
      true
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn format_atoms() {
    assert_eq!("3.45", &format!("{}", Value::Real(3.45)));
    assert_eq!("69105", &format!("{}", Value::Integer(69105)));
    assert_eq!("#f", &format!("{}", Value::Boolean(false)));
    assert_eq!("#t", &format!("{}", Value::Boolean(true)));
    assert_eq!("#\\newline", &format!("{}", Value::Character('\n')));
    assert_eq!("#\\x", &format!("{}", Value::Character('x')));
    assert_eq!("abc", &format!("{}", Value::Symbol("abc".to_string())));
    assert_eq!("\"abc\"", &format!("{}", Value::String("abc".to_string())));
  }

  #[test]
  fn format_list() {
    assert_eq!("()", &format!("{}", Value::EmptyList));
    assert_eq!("(=>1 . =>2)", &format!("{}", Value::Pair(
      RefCell::new(1), RefCell::new(2))
    ));
  }

  #[test]
  fn format_vec() {
    assert_eq!("#()", &format!("{}", Value::Vector(vec![])));
    assert_eq!("#(=>1 =>2)",
               &format!("{}", Value::Vector(vec![RefCell::new(1), RefCell::new(2)])));
  }
}