use arena::Arena;
use eval::evaluate;
use value::Value;

#[derive(Debug)]
pub enum Bounce {
    Evaluate {
        value: usize,
        environment: usize,
        continuation: usize,
    },
    Resume {
        value: usize,
        continuation: usize,
    },
    Done(usize),
}

impl Bounce {
    pub fn run_trampoline(self, arena: &mut Arena) -> Result<usize, String> {
        let mut current_bounce = self;
        loop {
            // println!(" C> {:?}", &current_bounce);
            match current_bounce {
                Bounce::Evaluate {
                    value,
                    environment,
                    continuation,
                } => {
                    current_bounce = evaluate(arena, value, environment, continuation)?;
                }
                Bounce::Resume {
                    value,
                    continuation,
                } => {
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

pub fn evaluate_toplevel(
    arena: &mut Arena,
    value: usize,
    continuation: usize,
    environment: usize,
) -> Result<usize, String> {
    evaluate(arena, value, continuation, environment)?.run_trampoline(arena)
}
