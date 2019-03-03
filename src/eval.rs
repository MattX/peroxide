use std::cell::Ref;
use std::cell::RefCell;

use arena::Arena;
use continuation::Continuation;
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
    Value::Pair(_, _) => evaluate_pair(arena, environment, val, continuation),
    _ => Bounce::Resume { continuation_r: continuation, value_r: form },
  }.run_trampoline(arena)
}

fn evaluate_variable(arena: &mut Arena, environment: usize, name: &str, continuation: usize)
                     -> Bounce {
  if let Value::Environment(e) = arena.value_ref(environment) {
    match e.borrow().get(arena, name) {
      Some(v) => Bounce::Resume { continuation_r: continuation, value_r: v },
      None => Bounce::Done(Err(format!("Undefined value: {}.", name))),
    }
  } else {
    panic!("Value passed to evaluate_variable is not an environment: {:?}", arena.value_ref(environment));
  }
}

fn evaluate_pair(arena: &mut Arena, environment: usize, pair: Value, continuation: usize)
                 -> Bounce {
  if let Value::Pair(car_r, cdr_r) = pair {
    let car = arena.value_ref(*car_r.borrow()).clone();
    if let Value::Symbol(s) = car {
      match s.as_ref() {
        "quote" => evaluate_quote(arena, environment, *cdr_r.borrow(), continuation),
        "if" => Bounce::Done(Err("'if' not implemented".to_string())),
        "begin" => Bounce::Done(Err("'begin' not implemented".to_string())),
        "lambda" => Bounce::Done(Err("'lambda' not implemented".to_string())),
        "set!" => Bounce::Done(Err("'set!' not implemented".to_string())),
        _ => Bounce::Done(Err("application not implemented".to_string())),
      }
    } else {
      Bounce::Done(Err("application not implemented".to_string()))
    }
  } else {
    panic!("Value passed to evaluate_pair() is not a pair: {:?}", pair);
  }
}


fn evaluate_quote(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize)
                  -> Bounce {
  let cdr = arena.value_ref(cdr_r);
  if let Value::Pair(cdar_r, cddr_r) = cdr {
    let cddr = arena.value_ref(*cddr_r.borrow());
    if let Value::EmptyList = cddr {
      Bounce::Resume { continuation_r: continuation, value_r: *cdar_r.borrow() }
    } else {
      Bounce::Done(Err(format!("Syntax error in quote.")))
    }
  } else {
    Bounce::Done(Err(format!("Syntax error in quote.")))
  }
}


fn resume_cont(arena: &mut Arena, form: usize, continuation: usize) -> Bounce {
  if let Value::Continuation(c) = arena.value_ref(continuation).clone() {
    c.borrow().clone().resume(arena, form)
  } else {
    panic!("Value passed to resume_cont() is not a continuation: {:?}", arena.value_ref(continuation));
  }
}

fn bounce_new(a: &mut Arena, c: Continuation, v: usize) -> Bounce {
  let c_r = a.intern(Value::Continuation(RefCell::new(c)));
  Bounce::Resume { continuation_r: c_r, value_r: v }
}
