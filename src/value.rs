use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;

use arena::Arena;
use environment::Environment;

#[derive(Debug, PartialEq)]
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
      Value::Environment(e) => write!(f, "{:?}", e)
    }
  }
}

pub fn pretty_print(arena: &Arena, p: &Value) -> String {
  match p {
    Value::Pair(_, _) => print_pair(arena, p),
    Value::Vector(_) => print_vector(arena, p),
    _ => format!("{}", p)
  }
}

fn print_pair(arena: &Arena, p: &Value) -> String {
  fn _print_pair(arena: &Arena, p: &Value, s: &mut String) {
    match p {
      Value::Pair(a, b) => {
        s.push_str(&pretty_print(arena, arena.value_ref(*a.borrow().deref()))[..]);
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

  match p {
    Value::Pair(_, _) | Value::EmptyList => {
      let mut s = "(".to_string();
      _print_pair(arena, p, &mut s);
      s
    }
    _ => panic!("print_pair passed a value that is not a pair: {:?}.", p)
  }
}

fn print_vector(arena: &Arena, v: &Value) -> String {
  if let Value::Vector(vals) = v {
    let contents = vals.iter()
        .map(|e| format!("{}", pretty_print(arena, arena.value_ref(*e.borrow().deref()))))
        .collect::<Vec<String>>()
        .join(" ");
    format!("#({})", contents)
  } else {
    panic!("print_vector passed a value that is not a vector: {:?}.", v)
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