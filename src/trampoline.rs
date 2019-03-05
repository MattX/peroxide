use arena::Arena;
use eval::{evaluate, evaluate_begin};
use value::Value;
use continuation::Continuation;


#[derive(Debug)]
pub enum Bounce {
  Evaluate { value: usize, environment: usize, continuation: usize },
  EvaluateBegin { value: usize, environment: usize, continuation: usize },
  EvaluateArguments { args: usize, environment: usize, continuation: usize },
  Resume { value: usize, continuation: usize },
  Done(usize),
}

impl Bounce {
  pub fn run_trampoline(self, arena: &mut Arena) -> Result<usize, String> {
    let mut current_bounce = self;
    loop {
      // println!(" C> {:?}", &current_bounce);
      match current_bounce {
        Bounce::Evaluate { value, environment, continuation } => {
          current_bounce = evaluate(arena, value, environment, continuation)?;
        },
        Bounce::EvaluateBegin { value, environment, continuation } => {
          current_bounce = evaluate_begin(arena, value, environment, continuation)?;
        }
        Bounce::EvaluateArguments { args, environment, continuation } => {
          current_bounce = evaluate_arguments(arena, args, environment, continuation)?;
        }
        Bounce::Resume { value, continuation } => {
          if let Value::Continuation(c) = arena.value_ref(continuation).clone() {
            current_bounce = c.borrow().resume(arena, value)?;
          } else {
            panic!("Resuming non-continuation.")
          }
        }
        Bounce::Done(u) => return Ok(u),
      }
    }
  }
}

pub fn evaluate_toplevel(arena: &mut Arena, value: usize, continuation: usize, environment: usize)
                         -> Result<usize, String> {
  evaluate(arena, value, continuation, environment)?.run_trampoline(arena)
}

fn evaluate_arguments(arena: &mut Arena, args_r: usize, environment: usize, continuation: usize)
                      -> Result<Bounce, String> {
  let args = arena.value_ref(args_r).pair_to_vec(arena)
      .expect(&format!("Argument evaluation didn't produce a list."));

  if args.is_empty() {
    Ok(Bounce::Resume { value: arena.empty_list, continuation })
  } else {
    let cont = arena.intern_continuation(Continuation::Argument {
      sequence: args_r,
      environment,
      continuation,
    });
    Ok(Bounce::Evaluate { value: args[0], environment, continuation: cont })
  }
}