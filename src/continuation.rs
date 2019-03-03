use arena::Arena;
use trampoline::Bounce;
use value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true_r: usize, e_false_r: usize, environment_r: usize, next_r: usize },
  Begin { body_r: usize, environment_r: usize, next_r: usize },
  Set { name: String, environment_r: usize, next_r: usize, define: bool },
  EvFun { body_r: usize, environment_r: usize, next_r: usize },
  Apply { fun_r: usize, environment_r: usize, next_r: usize },
  Argument { sequence_r: usize, environment_r: usize, next_r: usize },
  Gather { value_r: usize, next_r: usize },
  TopLevel,
}

impl Continuation {
  pub fn resume(&self, arena: &mut Arena, value_r: Option<usize>) -> Bounce {
    let value = value_r.map(|v| arena.value_ref(v));
    match self {
      Continuation::If { e_true_r, e_false_r, environment_r, next_r } => {
        match value {
          Some(v) => {
            Bounce::Evaluate {
              continuation_r: *next_r,
              value_r: if v.truthy() { *e_true_r } else { *e_false_r },
              environment_r: *environment_r,
            }
          }
          None => Bounce::Done(Err(format!("No value returned by 'if' predicate.")))
        }
      },
      Continuation::Begin { body_r, environment_r, next_r } => {
        Bounce::EvaluateBegin { value_r: *body_r, environment_r: *environment_r, continuation_r: *next_r }
      },
      Continuation::Set { name, environment_r, next_r, define } => {
        match arena.value_ref(*environment_r) {
          Value::Environment(e) => {
            match value_r {
              Some(v) => {
                if *define {
                  e.borrow_mut().define(name, v);
                  Bounce::Done(Ok(None))
                } else {
                  match e.borrow_mut().set(arena, name, v) {
                    Ok(_) => Bounce::Done(Ok(None)),
                    Err(s) => Bounce::Done(Err(format!("Cannot set {}: {}", name, s)))
                  }
                }
              }
              None => Bounce::Done(Err(format!("Cannot set {} to unspecific value.", name)))
            }
          }
          _ => panic!("Expected environment, got {}.", arena.value_ref(*environment_r).pretty_print(arena))
        }
      }
      Continuation::TopLevel => Bounce::Done(Ok(value_r)),
      _ => Bounce::Done(Err(format!("Not implemented")))
    }
  }
}
