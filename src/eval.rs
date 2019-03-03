use std::cell::Ref;

use arena::Arena;
use environment::Environment;
use trampoline::Bounce;
use value::Value;

pub fn evaluate(arena: &mut Arena, form: usize, environment: usize, continuation: usize)
                -> Result<usize, String> {
  if let Value::Environment(e) = arena.value_ref(environment) {} else {
    panic!("Value passed to evaluate() is not an environment: {:?}", arena.value_ref(environment));
  }

  let val = arena.value_ref(form).clone();
  match val {
    Value::Symbol(s) => evaluate_variable(arena, environment, &s, continuation),
    _ => Bounce::Resume { continuation_r: continuation, value_r: form },
  }.run_trampoline(arena)
}

fn evaluate_variable(arena: &mut Arena, environment: usize, name: &str, continuation: usize)
                     -> Bounce {
  if let Value::Environment(e) = arena.value_ref(environment) {
    match  e.borrow().get(arena, name) {
      Some(v) => Bounce::Resume { continuation_r: continuation, value_r: v },
      None => Bounce::Done(Err(format!("Undefined value: {}.", name))),
    }
  } else {
    panic!("Value passed to evaluate_variable is not an environment: {:?}", arena.value_ref(environment));
  }
}

fn resume_cont(arena: &mut Arena, form: usize, continuation: usize) -> Bounce {
  if let Value::Continuation(c) = arena.value_ref(continuation).clone() {
    c.borrow().clone().resume(arena, form)
  } else {
    panic!("Value passed to resume_cont() is not a continuation: {:?}", arena.value_ref(continuation));
  }
}