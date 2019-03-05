use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;

use arena::Arena;
use continuation::Continuation;
use environment::Environment;
use primitives::Primitive;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
  Unspecific,
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
  Lambda { environment: usize, formals: usize, body: usize },
  Primitive(Primitive)
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Value::Unspecific => write!(f, "#unspecific"),
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
        _ => return Err(format!("Converting list to vec: {} is not a proper list",
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

  pub fn cdr(&self) -> usize {
    if let Value::Pair(_, cdr) = self {
      *cdr.borrow()
    } else {
      panic!("Not a pair: {:?}.", self)
    }
  }

  pub fn bind_formals(&self, arena: &Arena, args: usize) -> Result<Vec<(String, usize)>, String> {
    fn _bind_formals(arena: &Arena, formals: usize, args: usize) -> Result<Vec<(String, usize)>, String> {
      let fm = arena.value_ref(formals);
      let act = arena.value_ref(args);
      match (fm, act) {
        (Value::Symbol(s), _) => Ok(vec![(s.clone(), args)]),
        (Value::EmptyList, Value::EmptyList) => Ok(vec![]),
        (Value::Pair(f_car, f_cdr), Value::Pair(a_car, a_cdr)) => {
          let f_car_v = arena.value_ref(*f_car.borrow());
          if let Value::Symbol(s) = f_car_v {
            let mut rest = _bind_formals(arena, *f_cdr.borrow(), *a_cdr.borrow())?;
            rest.push((s.clone(), *a_car.borrow()));
            Ok(rest)
          } else {
            // TODO turn this into a panic once we start checkings
            Err(format!("Malformed formals, expected symbol, got {}.", f_car_v))
          }
        }
        _ => Err(format!("Malformed formals ({}), or formals do not match argument list ({}).",
                         fm.pretty_print(arena), act.pretty_print(arena)))
      }
    }

    if let Value::Lambda { environment: _, formals, body: _ } = self {
      _bind_formals(arena, *formals, args)
    } else {
      panic!("bind_formals called on {:?}.", self)
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