use std::fmt::{Debug, Error, Formatter};
use value::Value;
use arena::Arena;

pub enum Bounce {
  Resume { continuation_r: usize, value_r: usize },
  Done(Result<usize, String>),
}

impl Debug for Bounce {
  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
    match self {
      Bounce::Resume { continuation_r, value_r } => write!(f, "Resume()"),
      Bounce::Done(u) => write!(f, "Done({:?})", u)
    }
  }
}

impl Bounce {
  pub fn run_trampoline(self, arena: &mut Arena) -> Result<usize, String> {
    let mut current_bounce = self;
    loop {
      match current_bounce {
        Bounce::Resume { continuation_r, value_r } => {
          if let Value::Continuation(c) = arena.value_ref(continuation_r).clone() {
            current_bounce = c.borrow().resume(arena, value_r);
          } else {
            panic!("Resuming non-continuation.")
          }
        }
        Bounce::Done(u) => return u,
      }
    }
  }
}
