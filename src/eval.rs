use std::cell::RefCell;

use arena::Arena;
use continuation::Continuation;
use trampoline::Bounce;
use value::Value;

pub fn evaluate(arena: &mut Arena, form: usize, environment: usize, continuation: usize)
                -> Bounce {
  if let Value::Environment(e) = arena.value_ref(environment) {} else {
    panic!("Value passed to evaluate() is not an environment: {:?}", arena.value_ref(environment));
  }

  let val = arena.value_ref(form).clone();
  match val {
    Value::Symbol(s) => evaluate_variable(arena, environment, &s, continuation),
    Value::Pair(_, _) => evaluate_pair(arena, environment, val, continuation),
    _ => Bounce::Resume { continuation_r: continuation, value_r: Some(form) },
  }
}

fn evaluate_variable(arena: &mut Arena, environment: usize, name: &str, continuation: usize)
                     -> Bounce {
  if let Value::Environment(e) = arena.value_ref(environment) {
    match e.borrow().get(arena, name) {
      Some(v) => Bounce::Resume { continuation_r: continuation, value_r: Some(v) },
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
        "if" => evaluate_if(arena, environment, *cdr_r.borrow(), continuation),
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
  let lst = arena.value_ref(cdr_r).pair_to_vec(arena);

  match lst {
    Ok(l) => if l.len() != 1 {
      Bounce::Done(Err(format!("Syntax error in quote, expecting exactly 1 quoted value.")))
    } else {
      Bounce::Resume { continuation_r: continuation, value_r: Some(l[0]) }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in quote: {}.", s)))
  }
}


fn evaluate_if(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize)
               -> Bounce {
  let lst = arena.value_ref(cdr_r).pair_to_vec(arena);

  match lst {
    Ok(l) => if l.len() != 3 {
      Bounce::Done(Err(format!("Syntax error in if, expecting exactly 3 forms.")))
    } else {
      let cont = Continuation::If {
        e_true_r: l[1],
        e_false_r: l[2],
        environment_r: environment,
        next_r: continuation,
      };
      let cont_r = arena.intern(Value::Continuation(RefCell::new(cont)));
      Bounce::Evaluate { continuation_r: cont_r, value_r: l[0], environment_r: environment }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in if: {}.", s)))
  }
}

fn resume_cont(arena: &mut Arena, value: Option<usize>, continuation: usize) -> Bounce {
  if let Value::Continuation(c) = arena.value_ref(continuation).clone() {
    c.borrow().clone().resume(arena, value)
  } else {
    panic!("Value passed to resume_cont() is not a continuation: {:?}", arena.value_ref(continuation));
  }
}
