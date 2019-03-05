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
    match self {
      Continuation::If { e_true, e_false, environment, continuation } => {
        Ok(Bounce::Evaluate {
          continuation_r: *continuation,
          value_r: if value.truthy() { *e_true } else { *e_false },
          environment_r: *environment,
        })
      }

      Continuation::Begin { body, environment, continuation } => {
        Ok(Bounce::EvaluateBegin { value_r: *body, environment_r: *environment, continuation_r: *continuation })
      }

      Continuation::Set { name, define, environment, continuation } => {
        match arena.value_ref(*environment) {
          Value::Environment(e) => {
            let success_cont = Ok(Bounce::Resume { continuation_r: *continuation, value_r: arena.unspecific });
            if *define {
              e.borrow_mut().define(name, value_r);
              success_cont
            } else {
              match e.borrow_mut().set(arena, name, value_r) {
                Ok(_) => success_cont,
                Err(s) => Err(format!("Cannot set {}: {}", name, s))
              }
            }
          }
          _ => panic!("Expected environment, got {}.", arena.value_ref(*environment).pretty_print(arena))
        }
      }

      Continuation::EvFun { args, environment, continuation } => {
        let apply_cont = arena.intern_continuation(Continuation::Apply {
          fun: value_r,
          environment: *environment,
          continuation: *continuation,
        });
        Ok(Bounce::EvaluateArguments { args_r: *args, environment_r: *environment, continuation_r: apply_cont })
      }

      Continuation::Argument { sequence, environment, continuation } => {
        let gather_cont = arena.intern_continuation(Continuation::Gather {
          gathered: value_r,
          continuation: *continuation,
        });
        let cdr = arena.value_ref(*sequence).cdr();
        Ok(Bounce::EvaluateArguments { args_r: cdr, environment_r: *environment, continuation_r: gather_cont })
      }

      Continuation::Gather { gathered, continuation } => {
        let collected = arena.intern(Value::Pair(RefCell::new(*gathered), RefCell::new(value_r)));
        Ok(Bounce::Resume { continuation_r: *continuation, value_r: collected })
      }

      Continuation::Apply { fun: fun_r, environment: _, continuation } => {
        let fun = arena.value_ref(*fun_r).clone();
        if let Value::Lambda { environment: lambda_environment, formals: _, body } = fun {
          let bound_formals = fun.bind_formals(arena, value_r)
              .map_err(|s| format!("Error binding function arguments: {}.", s))?;

          let env = Environment::new_initial(Some(lambda_environment), bound_formals);
          let env_r = arena.intern(Value::Environment(RefCell::new(env)));
          Ok(Bounce::EvaluateBegin { environment_r: env_r, value_r: body, continuation_r: *continuation })
        } else {
          Err(format!("Tried to apply non-function: {}.", fun))
        }
      }

      Continuation::TopLevel => Ok(Bounce::Done(value_r)),
    }
  }
}
