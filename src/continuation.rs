use std::cell::RefCell;

use arena::Arena;
use trampoline::Bounce;
use value::Value;
use environment::Environment;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true_r: usize, e_false_r: usize, environment_r: usize, next_r: usize },
  Begin { body_r: usize, environment_r: usize, next_r: usize },
  Set { name: String, environment_r: usize, next_r: usize, define: bool },
  EvFun { args_r: usize, environment_r: usize, next_r: usize },
  Apply { fun_r: usize, environment_r: usize, next_r: usize },
  Argument { sequence_r: usize, environment_r: usize, next_r: usize },
  Gather { gathered_r: usize, next_r: usize },
  TopLevel,
}

impl Continuation {
  pub fn resume(&self, arena: &mut Arena, value_r: usize) -> Bounce {
    let value = arena.value_ref(value_r).clone();
    match self {
      Continuation::If { e_true_r, e_false_r, environment_r, next_r } => {
        Bounce::Evaluate {
          continuation_r: *next_r,
          value_r: if value.truthy() { *e_true_r } else { *e_false_r },
          environment_r: *environment_r,
        }
      }
      Continuation::Begin { body_r, environment_r, next_r } => {
        Bounce::EvaluateBegin { value_r: *body_r, environment_r: *environment_r, continuation_r: *next_r }
      }
      Continuation::Set { name, environment_r, next_r, define } => {
        match arena.value_ref(*environment_r) {
          Value::Environment(e) => {
            if *define {
              e.borrow_mut().define(name, value_r);
              Bounce::Resume { continuation_r: *next_r, value_r: arena.unspecific }
            } else {
              match e.borrow_mut().set(arena, name, value_r) {
                Ok(_) => Bounce::Resume { continuation_r: *next_r, value_r: arena.unspecific },
                Err(s) => Bounce::Done(Err(format!("Cannot set {}: {}", name, s)))
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
        Bounce::EvaluateArguments { args_r: *args_r, environment_r: *environment_r, continuation_r: apply_cont_r }
      }
      Continuation::Argument { sequence_r, environment_r, next_r } => {
        let gather_cont = Continuation::Gather {
          gathered_r: value_r,
          next_r: *next_r,
        };
        let gather_cont_r = arena.intern(Value::Continuation(RefCell::new(gather_cont)));
        let cdr = arena.value_ref(*sequence_r).cdr();
        Bounce::EvaluateArguments { args_r: cdr, environment_r: *environment_r, continuation_r: gather_cont_r }
      }
      Continuation::Gather { gathered_r, next_r } => {
        let gathered = Value::Pair(RefCell::new(*gathered_r), RefCell::new(value_r));
        let gathered_r = arena.intern(gathered);
        Bounce::Resume { continuation_r: *next_r, value_r: gathered_r }
      }
      Continuation::Apply { fun_r, environment_r: _, next_r } => {
        let fun = arena.value_ref(*fun_r).clone();
        if let Value::Lambda { environment: lambda_environment_r, formals: _, body } = fun {
          match fun.bind_formals(arena, value_r) {
            Ok(v) => {
              let mut env = Environment::new(Some(lambda_environment_r));
              env.define_all(v);
              let env_r = arena.intern(Value::Environment(RefCell::new(env)));
              Bounce::EvaluateBegin { environment_r: env_r, value_r: body, continuation_r:*next_r }
            },
            Err(s) => Bounce::Done(Err(format!("Error binding function arguments: {}.", s)))
          }
        } else {
          Bounce::Done(Err(format!("Tried to apply non-function: {}.", fun)))
        }
      }
      Continuation::TopLevel => Bounce::Done(Ok(value_r)),
    }
  }
}
