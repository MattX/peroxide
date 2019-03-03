use arena::Arena;
use eval::{evaluate, evaluate_begin};
use value::Value;
use eval::evaluate_arguments;


#[derive(Debug)]
pub enum Bounce {
  Evaluate { continuation_r: usize, value_r: usize, environment_r: usize },
  EvaluateBegin { continuation_r: usize, value_r: usize, environment_r: usize },
  EvaluateArguments { continuation_r: usize, args_r: usize, environment_r: usize },
  Resume { continuation_r: usize, value_r: usize },
  Done(Result<usize, String>),
}

impl Bounce {
  pub fn run_trampoline(self, arena: &mut Arena) -> Result<usize, String> {
    let mut current_bounce = self;
    loop {
      // println!(" C> {:?}", &current_bounce);
      match current_bounce {
        Bounce::Evaluate { continuation_r, value_r, environment_r } => {
          current_bounce = evaluate(arena, value_r, environment_r, continuation_r);
        },
        Bounce::EvaluateBegin { continuation_r, value_r, environment_r } => {
          current_bounce = evaluate_begin(arena, environment_r, value_r, continuation_r);
        },
        Bounce::EvaluateArguments { continuation_r, args_r, environment_r } => {
          current_bounce = evaluate_arguments(arena, environment_r, args_r, continuation_r)
        }
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
                         -> Result<usize, String> {
  evaluate(arena, value_r, continuation_r, environment_r).run_trampoline(arena)
}