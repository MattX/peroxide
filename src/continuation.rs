use std::cell::RefCell;

use arena::Arena;
use environment::Environment;
use eval::{evaluate_arguments, evaluate_begin};
use trampoline::Bounce;
use util::with_check_len;
use value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Continuation {
    If {
        e_true: usize,
        e_false: Option<usize>,
        environment: usize,
        continuation: usize,
    },
    Begin {
        body: Vec<usize>,
        environment: usize,
        continuation: usize,
    },
    Set {
        name: String,
        define: bool,
        environment: usize,
        continuation: usize,
    },
    EvFun {
        args: usize,
        environment: usize,
        continuation: usize,
    },
    Apply {
        fun: usize,
        environment: usize,
        continuation: usize,
    },
    Argument {
        sequence: usize,
        environment: usize,
        continuation: usize,
    },
    Gather {
        gathered: usize,
        continuation: usize,
    },
    TopLevel,
}

impl Continuation {
    pub fn resume(&self, arena: &mut Arena, value_r: usize) -> Result<Bounce, String> {
        let value = arena.value_ref(value_r).clone();
        match *self {
            Continuation::If {
                e_true,
                e_false,
                environment,
                continuation,
            } => Ok(Bounce::Evaluate {
                value: if value.truthy() {
                    e_true
                } else {
                    e_false.unwrap_or(arena.unspecific)
                },
                environment,
                continuation,
            }),

            Continuation::Begin {
                ref body,
                environment,
                continuation,
            } => evaluate_begin(arena, body, environment, continuation),

            Continuation::Set {
                ref name,
                define,
                environment,
                continuation,
            } => match arena.value_ref(environment) {
                Value::Environment(e) => {
                    let success_cont = Ok(Bounce::Resume {
                        value: arena.unspecific,
                        continuation,
                    });
                    if define {
                        e.borrow_mut().define(name, value_r);
                        success_cont
                    } else {
                        match e.borrow_mut().set(arena, name, value_r) {
                            Ok(_) => success_cont,
                            Err(s) => Err(format!("Cannot set {}: {}", name, s)),
                        }
                    }
                }
                _ => panic!(
                    "Expected environment, got {}.",
                    arena.value_ref(environment).pretty_print(arena)
                ),
            },

            Continuation::EvFun {
                args,
                environment,
                continuation,
            } => {
                let apply_cont = arena.intern_continuation(Continuation::Apply {
                    fun: value_r,
                    environment,
                    continuation,
                });
                evaluate_arguments(arena, args, environment, apply_cont)
            }

            Continuation::Argument {
                sequence,
                environment,
                continuation,
            } => {
                let gather_cont = arena.intern_continuation(Continuation::Gather {
                    gathered: value_r,
                    continuation,
                });
                let args = arena.value_ref(sequence).cdr();
                evaluate_arguments(arena, args, environment, gather_cont)
            }

            Continuation::Gather {
                gathered,
                continuation,
            } => {
                let collected = arena.intern_pair(gathered, value_r);
                Ok(Bounce::Resume {
                    value: collected,
                    continuation,
                })
            }

            Continuation::Apply {
                fun: fun_r,
                continuation,
                ..
            } => {
                let fun = arena.value_ref(fun_r).clone();
                let vec_args = arena
                    .value_ref(value_r)
                    .clone()
                    .pair_to_vec(arena)
                    .expect("Function arguments are not a list, which should never happen.");
                match fun {
                    Value::Lambda {
                        environment: lambda_environment,
                        formals,
                        body,
                    } => {
                        let bound_formals = formals
                            .bind(arena, &vec_args)
                            .map_err(|s| format!("Error binding function arguments: {}.", s))?;

                        let env = Environment::new_initial(Some(lambda_environment), bound_formals);
                        let env_r = arena.intern(Value::Environment(RefCell::new(env)));
                        evaluate_begin(arena, &body, env_r, continuation)
                    }

                    Value::Primitive(p) => {
                        let result = (p.implementation)(arena, vec_args)?;
                        Ok(Bounce::Resume {
                            value: result,
                            continuation,
                        })
                    }

                    Value::Continuation(_) => {
                        let vec_args = with_check_len(vec_args, Some(1), Some(1))
                            .map_err(|e| format!("Error when invoking continuation: {}.", e))?;
                        Ok(Bounce::Resume {
                            value: vec_args[0],
                            continuation: fun_r,
                        })
                    }

                    _ => Err(format!(
                        "Tried to apply non-function: {}.",
                        fun.pretty_print(arena)
                    )),
                }
            }

            Continuation::TopLevel => Ok(Bounce::Done(value_r)),
        }
    }
}

#[allow(dead_code)]
pub fn continuation_depth(arena: &Arena, c: usize) -> u64 {
    match arena.value_ref(c).clone() {
        Value::Continuation(Continuation::If { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::Apply { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::Gather { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::EvFun { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::Argument { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::Begin { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::Set { continuation, .. }) => {
            continuation_depth(arena, continuation) + 1
        }
        Value::Continuation(Continuation::TopLevel) => 0,
        _ => panic!("Wat"),
    }
}
