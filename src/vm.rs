// Copyright 2018-2019 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// TODO in this file: stop calling the activation frame an environment.

use arena::Arena;
use environment::{ActivationFrame, RcEnv};
use gc;
use primitives::PrimitiveImplementation;
use std::cell::RefCell;
use std::fmt::Write;
use value::{list_from_vec, pretty_print, vec_from_list, Value};

static MAX_RECURSION_DEPTH: usize = 1000;

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(usize),
    JumpFalse(usize),
    Jump(usize),
    GlobalArgumentSet { index: usize },
    GlobalArgumentGet { index: usize },
    CheckedGlobalArgumentGet { index: usize },
    DeepArgumentSet { depth: usize, index: usize },
    LocalArgumentGet { depth: usize, index: usize },
    CheckedLocalArgumentGet { depth: usize, index: usize },
    CheckArity { arity: usize, dotted: bool },
    ExtendEnv,
    Return,
    CreateClosure(usize),
    PackFrame(usize),
    ExtendFrame(usize),
    PreserveEnv,
    RestoreEnv,
    PushValue,
    PopFunction,
    FunctionInvoke { tail: bool },
    CreateFrame(usize),
    NoOp,
    Finish,
}

/// A struct to hold the VM instructions and a mapping of instruction to lexical environment.
///
/// This is essentially used to provide variable names when an undefined value is accessed.
///
/// Any given part of the bytecode corresponds to a statically-known lexical environment. `env_map`
/// keeps a vec of (bytecode address where an environment starts, environment), which we can
/// binary-search to get the current environment from the current pc.
// TODO: we don't need env_stack at runtime, so it should be moved to compile.rs.
#[derive(Debug)]
pub struct Code {
    instructions: Vec<Instruction>,
    env_map: Vec<(usize, RcEnv)>,
    env_stack: Vec<RcEnv>,
    lambda_map: Vec<(usize, Option<String>)>,
    lambda_stack: Vec<String>,
}

impl Code {
    pub fn new(global_environment: &RcEnv) -> Self {
        Code {
            instructions: vec![],
            env_map: vec![(0, global_environment.clone())],
            env_stack: vec![global_environment.clone()],
            lambda_map: vec![(0, None)],
            lambda_stack: vec![],
        }
    }

    pub fn push(&mut self, i: Instruction) {
        self.instructions.push(i);
    }

    pub fn replace(&mut self, index: usize, new: Instruction) {
        self.instructions[index] = new;
    }

    pub fn code_size(&self) -> usize {
        self.instructions.len()
    }

    pub fn push_env(&mut self, env: &RcEnv) {
        self.env_map.push((self.instructions.len(), env.clone()));
        self.env_stack.push(env.clone());
    }

    pub fn pop_env(&mut self) {
        let e = self
            .env_stack
            .pop()
            .expect("Popping environment with no environments on stack.");
        self.env_map.push((self.instructions.len(), e));
    }

    pub fn find_env(&self, at: usize) -> &RcEnv {
        let env_index = self
            .env_map
            .binary_search_by_key(&at, |(instr, _env)| *instr)
            .unwrap_or_else(|e| e - 1);
        &self.env_map[env_index].1
    }

    pub fn push_lambda(&mut self, l: &str) {
        self.lambda_map
            .push((self.instructions.len(), Some(l.into())));
        self.lambda_stack.push(l.into());
    }

    pub fn pop_lambda(&mut self) {
        let e = self.lambda_stack.pop();
        self.lambda_map.push((self.instructions.len(), e));
    }

    pub fn find_lambda(&self, at: usize) -> &Option<String> {
        let lambda_index = self
            .lambda_map
            .binary_search_by_key(&at, |(instr, _env)| *instr)
            .unwrap_or_else(|e| e - 1);
        &self.lambda_map[lambda_index].1
    }
}

#[derive(Debug)]
struct Vm<'a> {
    value: usize,
    code: &'a mut Code,
    pc: usize,
    return_stack: Vec<usize>,
    stack: Vec<usize>,
    global_env: usize,
    env: usize,
    fun: usize,
}

enum Error {
    Raise(usize),
    Abort(usize),
}

fn raise_string(arena: &Arena, error: String) -> Error {
    Error::Raise(arena.insert(Value::String(RefCell::new(error))))
}

pub fn run(arena: &Arena, code: &mut Code, pc: usize, env: usize) -> Result<usize, usize> {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc,
        return_stack: Vec::new(),
        stack: Vec::new(),
        global_env: env,
        env,
        fun: 0,
    };
    loop {
        match run_one(arena, &mut vm) {
            Ok(brk) => {
                if brk {
                    break;
                }
            }
            Err(Error::Abort(v)) => return Err(v),
            Err(Error::Raise(v)) => return Err(v),
        }
    }
    Ok(vm.value)
}

fn run_one(arena: &Arena, vm: &mut Vm) -> Result<bool, Error> {
    match vm.code.instructions[vm.pc] {
        Instruction::Constant(v) => vm.value = v,
        Instruction::JumpFalse(offset) => {
            if !arena.get(vm.value).truthy() {
                vm.pc += offset;
            }
        }
        Instruction::Jump(offset) => vm.pc += offset,
        Instruction::GlobalArgumentSet { index } => {
            get_activation_frame(arena, vm.global_env)
                .borrow_mut()
                .set(arena, 0, index, vm.value);
            vm.value = arena.unspecific;
        }
        Instruction::GlobalArgumentGet { index } => {
            vm.value = get_activation_frame(arena, vm.global_env)
                .borrow()
                .get(arena, 0, index);
        }
        Instruction::CheckedGlobalArgumentGet { index } => {
            vm.value = arena
                .get_activation_frame(vm.global_env)
                .borrow()
                .get(arena, 0, index);
            if vm.value == arena.undefined {
                return Err(raise_string(
                    arena,
                    error_stack(
                        vm,
                        format!(
                            "Variable used before definition: {}",
                            resolve_variable(vm.code, vm.pc, 0, index)
                        ),
                    ),
                ));
            }
        }
        Instruction::DeepArgumentSet { depth, index } => {
            get_activation_frame(arena, vm.env)
                .borrow_mut()
                .set(arena, depth, index, vm.value);
            vm.value = arena.unspecific;
        }
        Instruction::LocalArgumentGet { depth, index } => {
            vm.value = get_activation_frame(arena, vm.env)
                .borrow()
                .get(arena, depth, index);
        }
        Instruction::CheckedLocalArgumentGet { depth, index } => {
            let frame = arena.get_activation_frame(vm.env).borrow();
            vm.value = frame.get(arena, depth, index);
            if vm.value == arena.undefined {
                let current_depth = frame.depth(arena);
                return Err(raise_string(
                    arena,
                    error_stack(
                        vm,
                        format!(
                            "Variable used before definition: {}",
                            resolve_variable(vm.code, vm.pc, current_depth - depth, index)
                        ),
                    ),
                ));
            }
        }
        Instruction::CheckArity { arity, dotted } => {
            let actual_arity = get_activation_frame(arena, vm.value).borrow().values.len();
            if dotted && actual_arity < arity {
                return Err(raise_string(
                    arena,
                    error_stack(
                        vm,
                        format!(
                            "Expected at least {} arguments, got {}.",
                            arity, actual_arity
                        ),
                    ),
                ));
            } else if !dotted && actual_arity != arity {
                return Err(raise_string(
                    arena,
                    error_stack(
                        vm,
                        format!("Expected {} arguments, got {}.", arity, actual_arity),
                    ),
                ));
            }
        }
        Instruction::ExtendEnv => {
            get_activation_frame(arena, vm.value).borrow_mut().parent = Some(vm.env);
            vm.env = vm.value;
        }
        Instruction::Return => {
            vm.pc = vm
                .return_stack
                .pop()
                .expect("Returning with no values on return stack.");
        }
        Instruction::CreateClosure(offset) => {
            vm.value = arena.insert(Value::Lambda {
                code: vm.pc + offset,
                environment: vm.env,
            })
        }
        Instruction::PackFrame(arity) => {
            let mut borrowed_frame = get_activation_frame(arena, vm.value).borrow_mut();
            let frame_len = std::cmp::max(arity, borrowed_frame.values.len());
            let listified = list_from_vec(arena, &borrowed_frame.values[arity..frame_len]);
            borrowed_frame.values.resize(arity + 1, arena.undefined);
            borrowed_frame.values[arity] = listified;
        }
        Instruction::ExtendFrame(by) => {
            let mut frame = arena.get_activation_frame(vm.value).borrow_mut();
            let len = frame.values.len();
            frame.values.resize(len + by, arena.undefined);
        }
        Instruction::PreserveEnv => {
            vm.stack.push(vm.env);
        }
        Instruction::RestoreEnv => {
            let env_r = vm
                .stack
                .pop()
                .expect("Restoring env with no values on stack.");
            if let Value::ActivationFrame(_) = arena.get(env_r) {
                vm.env = env_r;
            } else {
                panic!("Restoring non-activation frame.");
            }
        }
        Instruction::PushValue => {
            vm.stack.push(vm.value);
        }
        Instruction::PopFunction => {
            let fun_r = vm
                .stack
                .pop()
                .expect("Popping function with no values on stack.");
            match arena.get(fun_r) {
                Value::Lambda { .. } | Value::Primitive(_) | Value::Continuation(_) => {
                    vm.fun = fun_r
                }
                _ => {
                    return Err(raise_string(
                        arena,
                        error_stack(
                            vm,
                            format!("Cannot pop non-function: {}", pretty_print(arena, fun_r)),
                        ),
                    ));
                }
            }
        }
        Instruction::FunctionInvoke { tail } => {
            invoke(arena, vm, tail).map_err(|e| raise_string(arena, error_stack(&vm, e)))?;
        }
        Instruction::CreateFrame(size) => {
            let mut frame = ActivationFrame {
                parent: None,
                values: vec![0; size],
            };
            for i in (0..size).rev() {
                frame.values[i] = vm.stack.pop().expect("Too few values on stack.");
            }
            vm.value = arena.insert(Value::ActivationFrame(RefCell::new(frame)));
        }
        Instruction::NoOp => panic!("NoOp encountered."),
        Instruction::Finish => return Ok(true),
    }
    vm.pc += 1;
    Ok(false)
}

// TODO remove this
fn get_activation_frame(arena: &Arena, env: usize) -> &RefCell<ActivationFrame> {
    arena.get_activation_frame(env)
}

fn invoke(arena: &Arena, vm: &mut Vm, tail: bool) -> Result<(), String> {
    let fun = arena.get(vm.fun);
    match fun {
        Value::Lambda {
            code, environment, ..
        } => {
            if !tail {
                if vm.return_stack.len() > MAX_RECURSION_DEPTH {
                    return Err("Maximum recursion depth exceeded".into());
                }
                vm.return_stack.push(vm.pc);
            }
            vm.env = *environment;
            vm.pc = *code;
        }
        Value::Primitive(p) => match p.implementation {
            PrimitiveImplementation::Simple(i) => {
                let af = arena.get_activation_frame(vm.value);
                let values = &af.borrow().values;
                vm.value = i(arena, &values).map_err(|e| format!("In {:?}: {}", p, e))?;
            }
            PrimitiveImplementation::Apply => apply(arena, vm, tail)?,
            PrimitiveImplementation::CallCC => call_cc(arena, vm)?,
            _ => return Err(format!("Unimplemented: {}", p.name)),
        },
        Value::Continuation(c) => {
            let af = arena.get_activation_frame(vm.value).borrow();
            if af.values.len() != 1 {
                return Err("Invoking continuation with more than one argument".into());
            }
            vm.stack = c.stack.clone();
            vm.return_stack = c.return_stack.clone();
            vm.pc = vm
                .return_stack
                .pop()
                .expect("Popping continuation with no return address");
            vm.value = af.values[0];
        }
        _ => {
            return Err(format!(
                "Cannot invoke non-function: {}",
                fun.pretty_print(arena)
            ));
        }
    }
    Ok(())
}

fn apply(arena: &Arena, vm: &mut Vm, tail: bool) -> Result<(), String> {
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args < 2 {
        return Err("apply: too few arguments.".into());
    }
    let mut values = af.values[1..n_args - 1].to_vec();
    values.extend(vec_from_list(arena, af.values[n_args - 1])?.into_iter());
    let new_af = ActivationFrame {
        parent: None,
        values,
    };
    vm.value = arena.insert(Value::ActivationFrame(RefCell::new(new_af)));
    vm.fun = af.values[0];
    invoke(arena, vm, tail)
}

fn call_cc(arena: &Arena, vm: &mut Vm) -> Result<(), String> {
    vm.return_stack.push(vm.pc);
    let cont = Continuation {
        stack: vm.stack.clone(),
        return_stack: vm.return_stack.clone(),
    };
    let cont_r = arena.insert(Value::Continuation(cont));
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args != 1 {
        return Err("%call/cc: expected a single argument".into());
    }
    let new_af = ActivationFrame {
        parent: None,
        values: vec![cont_r],
    };
    vm.value = arena.insert(Value::ActivationFrame(RefCell::new(new_af)));
    vm.fun = af.values[0];
    invoke(arena, vm, true)
}

fn resolve_variable(code: &Code, pc: usize, altitude: usize, index: usize) -> String {
    let env = code.find_env(pc).borrow();
    env.get_name(altitude, index)
}

fn error_stack(vm: &Vm, message: String) -> String {
    let mut message = message;
    let positions = std::iter::once(&vm.pc).chain(vm.return_stack.iter().rev());
    for ret in positions {
        let name = vm
            .code
            .find_lambda(*ret)
            .clone()
            .unwrap_or_else(|| "<toplevel>".into());
        write!(message, "\n\tat {}", name).unwrap();
    }
    message
}

#[derive(Debug, Clone, PartialEq)]
pub struct Continuation {
    stack: Vec<usize>,
    return_stack: Vec<usize>,
}

impl gc::Inventory for Continuation {
    fn inventory(&self, v: &mut gc::PushOnlyVec<usize>) {
        for obj in self.stack.iter() {
            v.push(*obj);
        }
    }
}
