use std::cell::RefCell;

use arena::Arena;
use environment::Environment;
use trampoline::Bounce;
use value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true: usize, e_false: usize, environment: usize, continuation: usize },
  Begin { body: usize, environment: usize, continuation: usize },
  Set { name: String, define: bool, environment: usize, continuation: usize },
  EvFun { args: usize, environment: usize, continuation: usize },
  Apply { fun: usize, environment: usize, continuation: usize },
  Argument { sequence: usize, environment: usize, continuation: usize },
  Gather { gathered: usize, continuation: usize },
  TopLevel,
}

impl Continuation {
  pub fn resume(&self, arena: &mut Arena, value_r: usize) -> Result<Bounce, String> {
    let value = arena.value_ref(value_r).clone();
    match *self {
      Continuation::If { e_true, e_false, environment, continuation } => {
        Ok(Bounce::Evaluate {
          value: if value.truthy() { e_true } else { e_false },
          environment,
          continuation,
        })
      }

      Continuation::Begin { body, environment, continuation } => {
        Ok(Bounce::EvaluateBegin { value: body, environment, continuation })
      }

      Continuation::Set { ref name, define, environment, continuation } => {
        match arena.value_ref(environment) {
          Value::Environment(e) => {
            let success_cont = Ok(Bounce::Resume { value: arena.unspecific, continuation });
            if define {
              e.borrow_mut().define(name, value_r);
              success_cont
            } else {
              match e.borrow_mut().set(arena, name, value_r) {
                Ok(_) => success_cont,
                Err(s) => Err(format!("Cannot set {}: {}", name, s))
              }
            }
          }
          _ => panic!("Expected environment, got {}.", arena.value_ref(environment).pretty_print(arena))
        }
      }

      Continuation::EvFun { args, environment, continuation } => {
        let apply_cont = arena.intern_continuation(Continuation::Apply {
          fun: value_r,
          environment,
          continuation,
        });
        Ok(Bounce::EvaluateArguments { args, environment, continuation: apply_cont })
      }

      Continuation::Argument { sequence, environment, continuation } => {
        let gather_cont = arena.intern_continuation(Continuation::Gather {
          gathered: value_r,
          continuation,
        });
        let args = arena.value_ref(sequence).cdr();
        Ok(Bounce::EvaluateArguments { args, environment, continuation: gather_cont })
      }

      Continuation::Gather { gathered, continuation } => {
        let collected = arena.intern_pair(gathered, value_r);
        Ok(Bounce::Resume { value: collected, continuation })
      }

      Continuation::Apply { fun: fun_r, environment: _, continuation } => {
        let fun = arena.value_ref(fun_r).clone();
        if let Value::Lambda { environment: lambda_environment, formals: _, body } = fun {
          let bound_formals = fun.bind_formals(arena, value_r)
              .map_err(|s| format!("Error binding function arguments: {}.", s))?;

          let env = Environment::new_initial(Some(lambda_environment), bound_formals);
          let env_r = arena.intern(Value::Environment(RefCell::new(env)));
          Ok(Bounce::EvaluateBegin { value: body, environment: env_r, continuation })
        } else {
          Err(format!("Tried to apply non-function: {}.", fun))
        }
      }

      Continuation::TopLevel => Ok(Bounce::Done(value_r)),
    }
  }
}
