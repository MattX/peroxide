use arena::Arena;
use continuation::Continuation;
use trampoline::Bounce;
use util::with_check_len;
use value::{Formals, Value};

pub fn evaluate(
    arena: &mut Arena,
    value: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    if let Value::Environment(_) = arena.value_ref(environment) {
    } else {
        panic!(
            "Value passed to evaluate() is not an environment: {:?}",
            arena.value_ref(environment)
        );
    }

    match arena.value_ref(value).clone() {
        Value::Symbol(s) => evaluate_variable(arena, &s, environment, continuation),
        Value::Pair(_, _) => evaluate_pair(arena, value, environment, continuation),
        Value::EmptyList => Err("Syntax error: applying empty list.".to_string()),
        _ => Ok(Bounce::Resume {
            value,
            continuation,
        }),
    }
}

pub fn evaluate_arguments(
    arena: &mut Arena,
    args_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let args = arena
        .value_ref(args_r)
        .pair_to_vec(arena)
        .unwrap_or_else(|_| panic!("Argument evaluation didn't produce a list."));

    if args.is_empty() {
        Ok(Bounce::Resume {
            value: arena.empty_list,
            continuation,
        })
    } else {
        let cont = arena.intern_continuation(Continuation::Argument {
            sequence: args_r,
            environment,
            continuation,
        });
        Ok(Bounce::Evaluate {
            value: args[0],
            environment,
            continuation: cont,
        })
    }
}

/// Evaluates a `begin` whose syntax hasn't been checked yet.
pub fn check_evaluate_begin(
    arena: &mut Arena,
    cdr_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let body = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .map_err(|s| format!("Syntax error in begin: {}.", s))?;
    evaluate_begin(arena, &body, environment, continuation)
}

pub fn evaluate_begin(
    arena: &mut Arena,
    body: &[usize],
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    match body.len() {
        0 => Ok(Bounce::Resume {
            value: arena.unspecific,
            continuation,
        }),
        1 => Ok(Bounce::Evaluate {
            value: body[0],
            environment,
            continuation,
        }),
        _ => {
            let cont = arena.intern_continuation(Continuation::Begin {
                body: body[1..].to_vec(),
                environment,
                continuation,
            });
            Ok(Bounce::Evaluate {
                value: body[0],
                environment,
                continuation: cont,
            })
        }
    }
}

fn evaluate_variable(
    arena: &mut Arena,
    name: &str,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    if let Value::Environment(e) = arena.value_ref(environment) {
        let value = e
            .borrow()
            .get(arena, name)
            .ok_or_else(|| format!("Undefined value: {}.", name))?;
        Ok(Bounce::Resume {
            value,
            continuation,
        })
    } else {
        panic!(
            "Value passed to evaluate_variable is not an environment: {:?}",
            arena.value_ref(environment)
        );
    }
}

fn evaluate_pair(
    arena: &mut Arena,
    pair_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let pair = arena.value_ref(pair_r).clone();

    if let Value::Pair(car_r, cdr_r) = pair {
        let car = arena.value_ref(*car_r.borrow()).clone();
        if let Value::Symbol(s) = car {
            match s.as_ref() {
                "quote" => evaluate_quote(arena, *cdr_r.borrow(), continuation),
                "if" => evaluate_if(arena, *cdr_r.borrow(), environment, continuation),
                "begin" => check_evaluate_begin(arena, *cdr_r.borrow(), environment, continuation),
                "lambda" => evaluate_lambda(arena, *cdr_r.borrow(), environment, continuation),
                "set!" => evaluate_set(arena, *cdr_r.borrow(), false, environment, continuation),
                "define" => evaluate_set(arena, *cdr_r.borrow(), true, environment, continuation),
                _ => evaluate_application(arena, pair_r, environment, continuation),
            }
        } else {
            evaluate_application(arena, pair_r, environment, continuation)
        }
    } else {
        panic!("Value passed to evaluate_pair() is not a pair: {:?}.", pair);
    }
}

fn evaluate_quote(arena: &mut Arena, cdr_r: usize, continuation: usize) -> Result<Bounce, String> {
    let args = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .and_then(|v| with_check_len(v, Some(1), Some(1)))
        .map_err(|s| format!("Syntax error in quote: {}.", s))?;

    Ok(Bounce::Resume {
        value: args[0],
        continuation,
    })
}

fn evaluate_if(
    arena: &mut Arena,
    cdr_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let args = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .and_then(|v| with_check_len(v, Some(2), Some(3)))
        .map_err(|s| format!("Syntax error in if: {}.", s))?;

    let cont = arena.intern_continuation(Continuation::If {
        e_true: args[1],
        e_false: args.get(2).cloned(),
        environment,
        continuation,
    });
    Ok(Bounce::Evaluate {
        value: args[0],
        environment,
        continuation: cont,
    })
}

fn evaluate_set(
    arena: &mut Arena,
    cdr_r: usize,
    define: bool,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let fn_name = if define { "define" } else { "set!" };
    let args = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .and_then(|v| with_check_len(v, Some(2), Some(2)))
        .map_err(|s| format!("Syntax error in {}: {}.", fn_name, s))?;

    let name = match arena.value_ref(args[0]) {
        Value::Symbol(s) => s.clone(),
        _ => {
            return Err(format!(
                "Expected symbol, got {}.",
                arena.value_ref(args[0]).pretty_print(arena)
            ));
        }
    };

    let cont = arena.intern_continuation(Continuation::Set {
        name,
        define,
        environment,
        continuation,
    });

    Ok(Bounce::Evaluate {
        value: args[1],
        environment,
        continuation: cont,
    })
}

// TODO (easy): verify formals at this point
fn evaluate_lambda(
    arena: &mut Arena,
    cdr_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let mut args = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .and_then(|v| with_check_len(v, Some(2), None))
        .map_err(|s| format!("Syntax error in lambda: {}.", s))?;
    let formals = args.remove(0);

    let val = Value::Lambda {
        environment,
        formals: Formals::new(arena, formals)?,
        body: args,
    };
    Ok(Bounce::Resume {
        value: arena.intern(val),
        continuation,
    })
}

fn evaluate_application(
    arena: &mut Arena,
    cdr_r: usize,
    environment: usize,
    continuation: usize,
) -> Result<Bounce, String> {
    let args = arena
        .value_ref(cdr_r)
        .pair_to_vec(arena)
        .map_err(|s| format!("Syntax error in application: {}.", s))?;

    if args.is_empty() {
        panic!("Syntax error in application: empty list. This should have been caught before.")
    }

    let fun_args = arena.value_ref(cdr_r).cdr();
    let cont = arena.intern_continuation(Continuation::EvFun {
        args: fun_args,
        environment,
        continuation,
    });
    Ok(Bounce::Evaluate {
        value: args[0],
        environment,
        continuation: cont,
    })
}
