use value;
use arena::Arena;
use trampoline::Bounce;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Continuation {
  pub next_r: Option<usize>, // Last continuation does not have a next
  pub typ: ContinuationType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ContinuationType {
  If { e_true_r: usize, e_false_r: usize, environment_r: usize },
  Begin { body_r: usize, environment_r: usize },
  Set { name: String, environment_r: usize },
  EvFun { body_r: usize, environment_r: usize },
  Apply { fun_r: usize, environment_r: usize },
  Argument { sequence_r: usize, environment_r: usize },
  Gather { value_r: usize },
  TopLevel,
}

impl Continuation {
  pub fn resume(&self, arena: &mut Arena, value: usize) -> Bounce {
    match &self.typ {
      ContinuationType::TopLevel => {
        Bounce::Done(Ok(value))
      },
      _ => Bounce::Done(Err(format!("Not implemented")))
    }
  }
}
