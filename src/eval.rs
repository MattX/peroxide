use std::cell::RefCell;

use arena::Arena;
use continuation::Continuation;
use trampoline::Bounce;
use value::Value;

pub fn evaluate(arena: &mut Arena, form: usize, environment: usize, continuation: usize)
                -> Bounce {
  if let Value::Environment(_) = arena.value_ref(environment) {} else {
    panic!("Value passed to evaluate() is not an environment: {:?}", arena.value_ref(environment));
  }

  let val = arena.value_ref(form).clone();
  match val {
    Value::Symbol(s) => evaluate_variable(arena, environment, &s, continuation),
    Value::Pair(_, _) => evaluate_pair(arena, environment, form, continuation),
    _ => Bounce::Resume { continuation_r: continuation, value_r: form },
  }
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

fn evaluate_pair(arena: &mut Arena, environment: usize, pair_r: usize, continuation: usize)
                 -> Bounce {
  let pair = arena.value_ref(pair_r).clone();

  if let Value::Pair(car_r, cdr_r) = pair {
    let car = arena.value_ref(*car_r.borrow()).clone();
    if let Value::Symbol(s) = car {
      match s.as_ref() {
        "quote" => evaluate_quote(arena, environment, *cdr_r.borrow(), continuation),
        "if" => evaluate_if(arena, environment, *cdr_r.borrow(), continuation),
        "begin" => evaluate_begin(arena, environment, *cdr_r.borrow(), continuation),
        "lambda" => evaluate_lambda(arena, environment, *cdr_r.borrow(), continuation),
        "set!" => evaluate_set(arena, environment, *cdr_r.borrow(), continuation, false),
        "define" => evaluate_set(arena, environment, *cdr_r.borrow(), continuation, true),
        _ => evaluate_application(arena, environment, pair_r, continuation),
      }
    } else {
      evaluate_application(arena, environment, pair_r, continuation)
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
      Bounce::Resume { continuation_r: continuation, value_r: l[0] }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in quote: {}.", s)))
  }
}

// TODO (easy: support 2-form version)
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


pub fn evaluate_begin(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize)
                      -> Bounce {
  let val = arena.value_ref(cdr_r).pair_to_vec(arena);

  match val {
    Ok(v) => match v.len() {
      0 => Bounce::Resume { continuation_r: continuation, value_r: arena.unspecific },
      1 => {
        Bounce::Evaluate { value_r: v[0], environment_r: environment, continuation_r: continuation }
      }
      _ => {
        let cdr = arena.value_ref(cdr_r).cdr();
        let cont = arena.intern(Value::Continuation(RefCell::new(Continuation::Begin {
          body_r: cdr,
          environment_r: environment,
          next_r: continuation,
        })));
        Bounce::Evaluate { value_r: v[0], environment_r: environment, continuation_r: cont }
      }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in begin: {}.", s)))
  }
}


fn evaluate_set(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize,
                define: bool) -> Bounce {
  let val = arena.value_ref(cdr_r).pair_to_vec(arena);

  match val {
    Ok(v) => if v.len() != 2 {
      Bounce::Done(Err(format!("Syntax error in set!, expecting exactly 2 forms.")))
    } else {
      let name = match arena.value_ref(v[0]) {
        Value::Symbol(s) => s.clone(),
        _ => return Bounce::Done(Err(format!("Expected symbol, got {}.", arena.value_ref(v[0]).pretty_print(arena))))
      };
      let cont = arena.intern(Value::Continuation(RefCell::new(Continuation::Set {
        name,
        environment_r: environment,
        next_r: continuation,
        define,
      })));
      Bounce::Evaluate { value_r: v[1], environment_r: environment, continuation_r: cont }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in {}: {}.", if define { "define" } else { "set!" }, s)))
  }
}

// TODO (easy): verify formals at this point
fn evaluate_lambda(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize)
                   -> Bounce {
  let val = arena.value_ref(cdr_r).pair_to_vec(arena);

  match val {
    Ok(v) => if v.len() < 2 {
      Bounce::Done(Err(format!("Syntax error in lambda, expecting at least 2 forms.")))
    } else {
      let val = Value::Lambda { environment, formals: v[0], body: arena.value_ref(cdr_r).cdr() };
      let val_r = arena.intern(val);
      Bounce::Resume { continuation_r: continuation, value_r: val_r }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in lambda: {}.", s)))
  }
}


fn evaluate_application(arena: &mut Arena, environment: usize, cdr_r: usize, continuation: usize)
                        -> Bounce {
  let val = arena.value_ref(cdr_r).pair_to_vec(arena);

  match val {
    Ok(v) => if v.is_empty() {
      Bounce::Done(Err(format!("Syntax error in application: empty list.")))
    } else {
      let cont = Continuation::EvFun {
        args_r: arena.value_ref(cdr_r).cdr(),
        environment_r: environment,
        next_r: continuation,
      };
      let cont_r = arena.intern(Value::Continuation(RefCell::new(cont)));
      Bounce::Evaluate { environment_r: environment, value_r: v[0], continuation_r: cont_r }
    },
    Err(s) => Bounce::Done(Err(format!("Syntax error in application: {}.", s)))
  }
}


pub fn evaluate_arguments(arena: &mut Arena, environment: usize, args: usize, continuation: usize)
                          -> Bounce {
  let val = arena.value_ref(args).pair_to_vec(arena);

  match val {
    Ok(v) => if v.is_empty() {
      Bounce::Resume { continuation_r: continuation, value_r: arena.empty_list }
    } else {
      let cont = Continuation::Argument {
        sequence_r: args,
        environment_r: environment,
        next_r: continuation,
      };
      let cont_r = arena.intern(Value::Continuation(RefCell::new(cont)));
      Bounce::Evaluate { environment_r: environment, value_r: v[0], continuation_r: cont_r }
    },
    Err(e) => panic!("Argument to evaluate arguments isn't a list, which should have been caught before.")
  }
}
