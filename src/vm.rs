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
use environment::ActivationFrame;
use primitives::PrimitiveImplementation;
use std::cell::RefCell;
use value::Value::Lambda;
use value::{list_from_vec, pretty_print, Value};

#[derive(Debug)]
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

#[derive(Debug)]
struct Vm<'a> {
    value: usize,
    code: &'a [Instruction],
    pc: usize,
    return_stack: Vec<usize>,
    stack: Vec<usize>,
    global_env: usize,
    env: usize,
    fun: usize,
}

pub fn run(arena: &Arena, code: &[Instruction], pc: usize, env: usize) -> Result<usize, String> {
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
        match vm.code[vm.pc] {
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
                vm.value = get_activation_frame(arena, vm.global_env)
                    .borrow()
                    .get(arena, 0, index);
                if vm.value == arena.undefined {
                    // TODO maintain a mapping and print variable name here.
                    return Err(format!("Variable used before definition: global {}", index));
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
                vm.value = get_activation_frame(arena, vm.env)
                    .borrow()
                    .get(arena, depth, index);
                if vm.value == arena.undefined {
                    // TODO maintain a mapping and print variable name here.
                    return Err(format!(
                        "Variable used before definition: {}/{}",
                        depth, index
                    ));
                }
            }
            Instruction::CheckArity { arity, dotted } => {
                let actual_arity = get_activation_frame(arena, vm.value).borrow().values.len();
                if dotted && actual_arity < arity {
                    return Err(format!(
                        "Expected at least {} arguments, got {}.",
                        arity, actual_arity
                    ));
                } else if !dotted && actual_arity != arity {
                    return Err(format!(
                        "Expected {} arguments, got {}.",
                        arity, actual_arity
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
                vm.value = arena.insert(Lambda {
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
                    Value::Lambda { .. } | Value::Primitive(_) => vm.fun = fun_r,
                    _ => {
                        return Err(format!(
                            "Cannot apply non-function: {}",
                            pretty_print(arena, fun_r)
                        ))
                    }
                }
            }
            Instruction::FunctionInvoke { tail } => {
                invoke(arena, &mut vm, tail)?;
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
            Instruction::NoOp => return Err("NoOp encountered.".into()),
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    Ok(vm.value)
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
                vm.return_stack.push(vm.pc);
            }
            vm.env = *environment;
            vm.pc = *code;
        }
        Value::Primitive(p) => match p.implementation {
            PrimitiveImplementation::Simple(i) => {
                let af = arena.get_activation_frame(vm.value);
                let values = &af.borrow().values;
                vm.value = i(arena, &values)?;
            }
            PrimitiveImplementation::Apply => apply(arena, vm, tail)?,
            _ => return Err(format!("Unimplemented: {}", p.name)),
        },
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
    if af.values.is_empty() {
        return Err("apply: no arguments.".into());
    }
    let new_af = ActivationFrame {
        parent: Some(vm.value),
        values: af.values[1..].to_vec(),
    };
    vm.value = arena.insert(Value::ActivationFrame(RefCell::new(new_af)));
    vm.fun = af.values[0];
    invoke(arena, vm, tail)
}
