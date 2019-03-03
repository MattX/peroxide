use std::fmt::{Debug, Error, Formatter};

use arena::Arena;
use eval::{evaluate, evaluate_begin};
use value::Value;

pub enum Bounce {
  Evaluate { continuation_r: usize, value_r: usize, environment_r: usize },
  EvaluateBegin { continuation_r: usize, value_r: usize, environment_r: usize },
  Resume { continuation_r: usize, value_r: Option<usize> },
  Done(Result<Option<usize>, String>),
}

impl Debug for Bounce {
  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
    match self {
      Bounce::Evaluate { continuation_r: _, value_r: _, environment_r: _ } => write!(f, "Evaluate()"),
      Bounce::EvaluateBegin { continuation_r: _, value_r: _, environment_r: _ } => write!(f, "EvaluateBegin()"),
      Bounce::Resume { continuation_r: _, value_r: _ } => write!(f, "Resume()"),
      Bounce::Done(u) => write!(f, "Done({:?})", u)
    }
  }
}

impl Bounce {
  pub fn run_trampoline(self, arena: &mut Arena) -> Result<Option<usize>, String> {
    let mut current_bounce = self;
    loop {
      match current_bounce {
        Bounce::Evaluate { continuation_r, value_r, environment_r } => {
          current_bounce = evaluate(arena, value_r, environment_r, continuation_r);
        },
        Bounce::EvaluateBegin { continuation_r, value_r, environment_r } => {
          current_bounce = evaluate_begin(arena, environment_r, value_r, continuation_r);
        },
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

pub fn evaluate_toplevel(arena: &mut Arena, value_r: usize, continuation_r: usize, environment_r: usize)
                         -> Result<Option<usize>, String> {
  evaluate(arena, value_r, continuation_r, environment_r).run_trampoline(arena)
}