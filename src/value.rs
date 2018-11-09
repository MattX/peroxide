use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum Value {
  Real(f64),
  Integer(i64),
  Boolean(bool),
  Character(char),
  Symbol(String),
  String(String),
  EmptyList,
  Pair(Rc<RefCell<Value>>, Rc<RefCell<Value>>),
  Vector(Vec<Value>),
  // We'll add some stuff here later
}
