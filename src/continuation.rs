use std::cell::RefCell;

use arena::Arena;
use trampoline::Bounce;
use value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true_r: usize, e_false_r: usize, environment_r: usize, next_r: usize },
  Begin { body_r: usize, environment_r: usize, next_r: usize },
  Set { name: String, environment_r: usize, next_r: usize, define: bool },
  EvFun { args_r: usize, environment_r: usize, next_r: usize },
  Apply { fun_r: usize, environment_r: usize, next_r: usize },
  Argument { sequence_r: usize, environment_r: usize, next_r: usize },
  Gather { value_r: usize, next_r: usize },
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
        Bounce::Done(Err(format!("Not implemented")))
      }
      Continuation::Gather { value_r, next_r } => {
        Bounce::Done(Err(format!("Not implemented")))
      }
      Continuation::Apply { fun_r, environment_r, next_r } => {
        Bounce::Done(Err(format!("Not implemented")))
      }
      Continuation::TopLevel => Bounce::Done(Ok(value_r)),
    }
  }
}
