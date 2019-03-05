use std::cell::RefCell;

use arena::Arena;
use environment::Environment;
use trampoline::Bounce;
use value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true: usize, e_false: usize, environment: usize, continuation: usize },
  Begin { body_r: usize, environment_r: usize, next_r: usize },
  Set { name: String, environment_r: usize, next_r: usize, define: bool },
  EvFun { args_r: usize, environment_r: usize, next_r: usize },
  Apply { fun_r: usize, environment_r: usize, next_r: usize },
  Argument { sequence_r: usize, environment_r: usize, next_r: usize },
  Gather { gathered_r: usize, next_r: usize },
  TopLevel,
}

impl Continuation {
  pub fn resume(&self, arena: &mut Arena, value_r: usize) -> Result<Bounce, String> {
    let value = arena.value_ref(value_r).clone();
    match self {
      Continuation::If { e_true, e_false, environment, continuation } => {
        Ok(Bounce::Evaluate {
          continuation_r: *continuation,
          value_r: if value.truthy() { *e_true } else { *e_false },
          environment_r: *environment,
        })
      }

      Continuation::Begin { body_r, environment_r, next_r } => {
        Ok(Bounce::EvaluateBegin { value_r: *body_r, environment_r: *environment_r, continuation_r: *next_r })
      }

      Continuation::Set { name, environment_r, next_r, define } => {
        match arena.value_ref(*environment_r) {
          Value::Environment(e) => {
            if *define {
              e.borrow_mut().define(name, value_r);
              Ok(Bounce::Resume { continuation_r: *next_r, value_r: arena.unspecific })
            } else {
              match e.borrow_mut().set(arena, name, value_r) {
                Ok(_) => Ok(Bounce::Resume { continuation_r: *next_r, value_r: arena.unspecific }),
                Err(s) => Err(format!("Cannot set {}: {}", name, s))
              }
            }
          }
          _ => panic!("Expected environment, got {}.", arena.value_ref(*environment_r).pretty_print(arena))
        }
      }

      Continuation::EvFun { args_r, environment_r, next_r } => {
        let apply_cont = Continuation::Apply {
          fun_r: value_r,
          environment_r: *environment_r,
          next_r: *next_r,
        };
        let apply_cont_r = arena.intern(Value::Continuation(RefCell::new(apply_cont)));
        Ok(Bounce::EvaluateArguments { args_r: *args_r, environment_r: *environment_r, continuation_r: apply_cont_r })
      }

      Continuation::Argument { sequence_r, environment_r, next_r } => {
        let gather_cont = Continuation::Gather {
          gathered_r: value_r,
          next_r: *next_r,
        };
        let gather_cont_r = arena.intern(Value::Continuation(RefCell::new(gather_cont)));
        let cdr = arena.value_ref(*sequence_r).cdr();
        Ok(Bounce::EvaluateArguments { args_r: cdr, environment_r: *environment_r, continuation_r: gather_cont_r })
      }

      Continuation::Gather { gathered_r, next_r } => {
        let gathered = Value::Pair(RefCell::new(*gathered_r), RefCell::new(value_r));
        let gathered_r = arena.intern(gathered);
        Ok(Bounce::Resume { continuation_r: *next_r, value_r: gathered_r })
      }

      Continuation::Apply { fun_r, environment_r: _, next_r } => {
        let fun = arena.value_ref(*fun_r).clone();
        if let Value::Lambda { environment: lambda_environment, formals: _, body } = fun {
          let bound_formals = fun.bind_formals(arena, value_r)
              .map_err(|s| format!("Error binding function arguments: {}.", s))?;

          let env = Environment::new_initial(Some(lambda_environment), bound_formals);
          let env_r = arena.intern(Value::Environment(RefCell::new(env)));
          Ok(Bounce::EvaluateBegin { environment_r: env_r, value_r: body, continuation_r: *next_r })
        } else {
          Err(format!("Tried to apply non-function: {}.", fun))
        }
      }

      Continuation::TopLevel => Ok(Bounce::Done(value_r)),
    }
  }
}
