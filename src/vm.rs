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

use arena::Arena;
use environment::ActivationFrame;
use std::cell::RefCell;
use value::Value::Lambda;
use value::{list_from_vec, Value};

#[derive(Debug)]
pub enum Instruction {
    Constant(usize),
    JumpFalse(usize),
    Jump(usize),
    DeepArgumentSet { depth: usize, index: usize },
    DeepArgumentGet { depth: usize, index: usize },
    CheckArity { arity: usize, dotted: bool },
    ExtendEnv,
    Return,
    CreateClosure(usize),
    PackFrame(usize),
    PreserveEnv,
    RestoreEnv,
    PushValue,
    PopFunction,
    FunctionInvoke { tail: bool },
    CreateFrame(usize),
    ExtendFrame,
    NoOp,
    Finish,
}

#[derive(Debug)]
struct Vm<'a> {
    value: usize,
    code: &'a [Instruction],
    pc: usize,
    return_stack: Vec<usize>,
    stack: Vec<usize>,
    env: usize,
    fun: usize,
}

pub fn run(
    arena: &mut Arena,
    code: &[Instruction],
    pc: usize,
    env: usize,
) -> Result<usize, String> {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc,
        return_stack: Vec::new(),
        stack: Vec::new(),
        env,
        fun: 0,
    };
    loop {
        match vm.code[vm.pc] {
            Instruction::Constant(v) => vm.value = v,
            Instruction::JumpFalse(offset) => {
                if !arena.get(vm.value).truthy() {
                    vm.pc += offset;
                }
            }
            Instruction::Jump(offset) => vm.pc += offset,
            Instruction::DeepArgumentSet { depth, index } => {
                if let Value::ActivationFrame(af) = arena.get(vm.env) {
                    af.borrow_mut().set(arena, depth, index, vm.value);
                    vm.value = arena.unspecific;
                } else {
                    panic!("Environment is not an activation frame.");
                }
            }
            Instruction::DeepArgumentGet { depth, index } => {
                if let Value::ActivationFrame(af) = arena.get(vm.env) {
                    vm.value = af.borrow().get(arena, depth, index)
                } else {
                    panic!("Environment is not an activation frame.");
                }
            }
            Instruction::CheckArity { arity, dotted } => {
                if let Value::ActivationFrame(af) = arena.get(vm.value) {
                    let actual = af.borrow().values.len();
                    if dotted && actual < arity {
                        return Err(format!(
                            "Expected at least {} arguments, got {}.",
                            arity, actual
                        ));
                    } else if !dotted && actual != arity {
                        return Err(format!("Expected {} arguments, got {}.", arity, actual));
                    }
                } else {
                    panic!("Checking arity: value is not an activation frame.");
                }
            }
            Instruction::ExtendEnv => {
                if let Value::ActivationFrame(af) = arena.get(vm.value) {
                    af.borrow_mut().parent = Some(vm.env);
                    vm.env = vm.value;
                } else {
                    panic!("Extending env: value is not an activation frame.");
                }
            }
            Instruction::Return => {
                vm.pc = vm
                    .return_stack
                    .pop()
                    .expect("Returning with no values on return stack.");
            }
            Instruction::CreateClosure(offset) => {
                vm.value = arena.insert(Lambda {
                    code: vm.pc + offset,
                    environment: vm.env,
                })
            }
            Instruction::PackFrame(arity) => {
                if let Value::ActivationFrame(af) = arena.get(vm.value) {
                    let mut borrowed_frame = af.borrow_mut();
                    let frame_len = std::cmp::max(arity, borrowed_frame.values.len());
                    let listified = list_from_vec(arena, &borrowed_frame.values[arity..frame_len]);
                    borrowed_frame.values.resize(arity + 1, 0);
                    borrowed_frame.values[arity] = listified;
                } else {
                    panic!("Packing non-activation frame.");
                }
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
                    Value::Lambda { .. } | Value::Primitive(_) => vm.fun = fun_r,
                    _ => panic!("Popping non-function."),
                }
            }
            Instruction::FunctionInvoke { tail } => {
                let fun = arena.get(vm.fun);
                match fun {
                    Value::Lambda {
                        code, environment, ..
                    } => {
                        if !tail {
                            vm.return_stack.push(vm.pc);
                        }
                        vm.env = *environment;
                        vm.pc = *code;
                    }
                    Value::Primitive(p) => {
                        if let Value::ActivationFrame(af) = arena.get(vm.value).clone() {
                            let values = &af.borrow().values;
                            vm.value = (p.implementation)(arena, &values)?;
                        } else {
                            panic!("Primitive called on non-activation frame.");
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Cannot invoke non-function: {}",
                            fun.pretty_print(arena)
                        ));
                    }
                }
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
            Instruction::ExtendFrame => {
                if let Value::ActivationFrame(af) = arena.get(vm.env) {
                    af.borrow_mut().values.push(vm.value);
                    vm.value = arena.unspecific;
                } else {
                    panic!("Environment is not an activation frame.");
                }
            }
            Instruction::NoOp => return Err("NoOp encountered.".into()),
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    Ok(vm.value)
}
