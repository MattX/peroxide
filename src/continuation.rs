use arena::Arena;
use trampoline::Bounce;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
  If { e_true_r: usize, e_false_r: usize, environment_r: usize, next_r: usize },
  Begin { body_r: usize, environment_r: usize, next_r: usize },
  Set { name: String, environment_r: usize, next_r: usize },
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
      }
      Continuation::TopLevel => Bounce::Done(Ok(value_r)),
      _ => Bounce::Done(Err(format!("Not implemented")))
    }
  }
}
