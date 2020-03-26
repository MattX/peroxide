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

use std::cell::{Cell, RefCell};
use std::fmt::Write;

use arena::Arena;
use arena::ValRef;
use environment::ActivationFrame;
use heap::{Inventory, PoolPtr, PtrVec, RootPtr};
use primitives::PrimitiveImplementation;
use value::{list_from_vec, pretty_print, vec_from_list, Value};
use VmState;
use {heap, parse_compile_run};

static MAX_RECURSION_DEPTH: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnPoint {
    pub code_block: PoolPtr,
    pub pc: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vm {
    value: ValRef,
    pc: usize,
    return_stack: Vec<ReturnPoint>,
    stack: Vec<ValRef>,
    global_env: ValRef,
    env: ValRef,
    fun: ValRef,
    root_code_block: PoolPtr,
    current_code_block: PoolPtr,
}

impl Vm {
    fn set_value(&mut self, v: ValRef) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(v.0.ok());
        }
        self.value = v;
    }

    fn get_return_point(&self) -> ReturnPoint {
        ReturnPoint {
            code_block: self.current_code_block,
            pc: self.pc,
        }
    }
}

impl Inventory for Vm {
    fn inventory(&self, v: &mut PtrVec) {
        v.push(self.value.0);
        v.push(self.global_env.0);
        v.push(self.env.0);
        v.push(self.fun.0);
        for s in self.stack.iter() {
            v.push(s.0);
        }
        v.push(self.root_code_block);
        v.push(self.current_code_block);
        for rp in self.return_stack.iter() {
            v.push(rp.code_block);
        }
    }
}

// TODO rename env around here to frame
pub fn run(
    arena: &Arena,
    code: RootPtr,
    pc: usize,
    global_env: ValRef,
    env: ValRef,
) -> Result<RootPtr, RootPtr> {
    let mut vm = Vm {
        value: arena.unspecific,
        pc,
        return_stack: Vec::new(),
        stack: Vec::new(),
        global_env,
        env,
        fun: arena.unspecific,
        root_code_block: code.pp(),
        current_code_block: code.pp(),
    };
    arena.root_vm(&vm);
    let res = loop {
        match run_one_instruction(arena, &mut vm) {
            Ok(true) => break Ok(arena.root(vm.value)),
            Ok(_) => (),
            Err(e) => break handle_error(arena, &mut vm, e),
        }
    };
    arena.unroot_vm();
    res
}

fn run_one_instruction(arena: &Arena, vm: &mut Vm) -> Result<bool, Error> {
    let code = arena.get_code_block(ValRef(vm.current_code_block));
    // println!("running {:?}, rst {:?}", code.instructions[vm.pc], vm.return_stack);
    match code.instructions[vm.pc] {
        Instruction::Constant(v) => vm.set_value(ValRef(code.constants[v])),
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
            vm.set_value(arena.unspecific);
        }
        Instruction::GlobalArgumentGet { index } => {
            vm.set_value(
                get_activation_frame(arena, vm.global_env)
                    .borrow()
                    .get(arena, 0, index),
            );
        }
        Instruction::CheckedGlobalArgumentGet { index } => {
            vm.set_value(
                arena
                    .get_activation_frame(vm.global_env)
                    .borrow()
                    .get(arena, 0, index),
            );
            if vm.value == arena.undefined {
                return Err(raise_string(
                    arena,
                    format!(
                        "Variable used before definition: {}",
                        resolve_variable(arena, vm, 0, index)
                    ),
                ));
            }
        }
        Instruction::DeepArgumentSet { depth, index } => {
            get_activation_frame(arena, vm.env)
                .borrow_mut()
                .set(arena, depth, index, vm.value);
            vm.set_value(arena.unspecific);
        }
        Instruction::LocalArgumentGet { depth, index } => {
            vm.set_value(
                get_activation_frame(arena, vm.env)
                    .borrow()
                    .get(arena, depth, index),
            );
        }
        Instruction::CheckedLocalArgumentGet { depth, index } => {
            let frame = arena.get_activation_frame(vm.env).borrow();
            vm.set_value(frame.get(arena, depth, index));
            if vm.value == arena.undefined {
                let current_depth = frame.depth(arena);
                return Err(raise_string(
                    arena,
                    format!(
                        "Variable used before definition: {}",
                        resolve_variable(arena, vm, current_depth - depth, index)
                    ),
                ));
            }
        }
        Instruction::CheckArity { arity, dotted } => {
            let actual_arity = get_activation_frame(arena, vm.value).borrow().values.len();
            if dotted && actual_arity < arity {
                return Err(raise_string(
                    arena,
                    format!(
                        "Expected at least {} arguments, got {}.",
                        arity, actual_arity
                    ),
                ));
            } else if !dotted && actual_arity != arity {
                return Err(raise_string(
                    arena,
                    format!("Expected {} arguments, got {}.", arity, actual_arity),
                ));
            }
        }
        Instruction::ExtendEnv => {
            get_activation_frame(arena, vm.value).borrow_mut().parent = Some(vm.env);
            vm.env = vm.value;
        }
        Instruction::Return => {
            let ReturnPoint { code_block, pc } = vm
                .return_stack
                .pop()
                .expect("Returning with no values on return stack.");
            vm.current_code_block = code_block;
            vm.pc = pc;
        }
        Instruction::CreateClosure(idx) => {
            vm.set_value(arena.insert(Value::Lambda {
                code: code.code_blocks[idx],
                frame: vm.env.0,
            }));
        }
        Instruction::PackFrame(arity) => {
            let frame = get_activation_frame(arena, vm.value);
            let values = frame.borrow_mut().values.clone();
            let frame_len = std::cmp::max(arity, values.len());
            let listified = list_from_vec(arena, &values[arity..frame_len]);
            frame.borrow_mut().values.resize(arity + 1, arena.undefined);
            frame.borrow_mut().values[arity] = listified;
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
                        format!("cannot pop non-function: {}", pretty_print(arena, fun_r)),
                    ));
                }
            }
        }
        Instruction::FunctionInvoke { tail } => {
            invoke(arena, vm, tail)?;
        }
        Instruction::CreateFrame(size) => {
            let mut frame = ActivationFrame {
                parent: None,
                values: vec![arena.unspecific; size],
            };
            // We could just pop values from the stack as we add them to the frame, but
            // this causes them to become unrooted, which is bad. So we copy the values,
            // then truncate the stack.
            // Matt from 3 months later - I don't see why the above is true. There's no
            // allocations in this block so it's fine to temporarily unroot these values.
            let stack_len = vm.stack.len();
            for i in (0..size).rev() {
                frame.values[i] = *vm
                    .stack
                    .get(stack_len - size + i)
                    .expect("too few values on stack.");
            }
            vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(frame))));
            vm.stack.truncate(stack_len - size);
        }
        Instruction::NoOp => panic!("NoOp encountered."),
        Instruction::Finish => return Ok(true),
    }
    vm.pc += 1;
    Ok(false)
}

// TODO remove this
fn get_activation_frame(arena: &Arena, env: ValRef) -> &RefCell<ActivationFrame> {
    arena.get_activation_frame(env)
}

fn invoke(arena: &Arena, vm: &mut Vm, tail: bool) -> Result<(), Error> {
    let fun = arena.get(vm.fun);
    match fun {
        Value::Lambda { code, frame } => {
            if !tail {
                if vm.return_stack.len() > MAX_RECURSION_DEPTH {
                    return Err(Error::Abort(arena.insert_rooted(Value::String(
                        RefCell::new("Maximum recursion depth exceeded".into()),
                    ))));
                }
                vm.return_stack.push(vm.get_return_point());
            }
            vm.env = ValRef(*frame);
            vm.current_code_block = *code;
            vm.pc = 0;
        }
        Value::Primitive(p) => match p.implementation {
            PrimitiveImplementation::Simple(i) => {
                let af = arena.get_activation_frame(vm.value);
                let values = &af.borrow().values;
                vm.set_value(
                    i(arena, &values)
                        .map_err(|e| raise_string(arena, format!("In {:?}: {}", p, e)))?,
                );
            }
            PrimitiveImplementation::Apply => apply(arena, vm, tail)?,
            PrimitiveImplementation::CallCC => call_cc(arena, vm)?,
            PrimitiveImplementation::Abort => return Err(raise(arena, vm, true)),
            PrimitiveImplementation::Raise => return Err(raise(arena, vm, false)),
            PrimitiveImplementation::Eval => eval(arena, vm)?,
        },
        Value::Continuation(c) => {
            let af = arena.get_activation_frame(vm.value).borrow();
            if af.values.len() != 1 {
                return Err(raise_string(
                    arena,
                    "Invoking continuation with more than one argument".into(),
                ));
            }
            vm.stack = c.stack.clone();
            vm.return_stack = c.return_stack.clone();
            let ReturnPoint { code_block, pc } = vm
                .return_stack
                .pop()
                .expect("Popping continuation with no return address");
            vm.current_code_block = code_block;
            vm.pc = pc;
            vm.set_value(af.values[0]);
        }
        _ => {
            return Err(raise_string(
                arena,
                format!("Cannot invoke non-function: {}", fun.pretty_print(arena)),
            ));
        }
    }
    Ok(())
}

fn apply(arena: &Arena, vm: &mut Vm, tail: bool) -> Result<(), Error> {
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args < 2 {
        return Err(raise_string(arena, "apply: too few arguments".into()));
    }
    let mut values = af.values[1..n_args - 1].to_vec();
    let vec = vec_from_list(arena, af.values[n_args - 1]).map_err(|e| raise_string(arena, e))?;
    values.extend(vec.into_iter());
    let new_af = ActivationFrame {
        parent: None,
        values,
    };
    vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(new_af))));
    vm.fun = af.values[0];
    invoke(arena, vm, tail)
}

fn call_cc(arena: &Arena, vm: &mut Vm) -> Result<(), Error> {
    vm.return_stack.push(vm.get_return_point());
    let cont = Continuation {
        stack: vm.stack.clone(),
        return_stack: vm.return_stack.clone(),
    };
    let cont_r = arena.insert(Value::Continuation(cont));
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args != 1 {
        return Err(raise_string(
            arena,
            "%call/cc: expected a single argument".into(),
        ));
    }
    let new_af = ActivationFrame {
        parent: None,
        values: vec![cont_r],
    };
    vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(new_af))));
    vm.fun = af.values[0];
    invoke(arena, vm, true)
}

fn eval(arena: &Arena, vm: &mut Vm) -> Result<(), Error> {
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args != 2 {
        return Err(raise_string(arena, "eval: expected 2 arguments".into()));
    }
    let expr = af.values[0];
    let env_descriptor = arena
        .try_get_string(af.values[1])
        .ok_or_else(|| {
            raise_string(
                arena,
                format!("eval: invalid environment descriptor: {}", &*af.values[1]),
            )
        })?
        .borrow()
        .clone();

    // TODO filter environment depending on env descriptor

    let res = parse_compile_run(arena, &mut VmState::new(arena), arena.root(expr))
        .map_err(|e| raise_string(arena, format!("eval: {}", e)))?;
    vm.set_value(res.vr());
    Ok(())
}

fn resolve_variable(arena: &Arena, vm: &Vm, altitude: usize, index: usize) -> String {
    let env = &arena
        .get_code_block(ValRef(vm.current_code_block))
        .environment;
    env.borrow().get_name(altitude, index)
}

fn raise(arena: &Arena, vm: &Vm, abort: bool) -> Error {
    let af = arena.get_activation_frame(vm.value).borrow();
    let n_args = af.values.len();
    if n_args != 1 {
        raise_string(arena, "raise: expected a single argument".into())
    } else if abort {
        Error::Abort(arena.root(af.values[0]))
    } else {
        Error::Raise(arena.root(af.values[0]))
    }
}

fn error_stack(arena: &Arena, vm: &Vm, error: Error) -> Error {
    let mut message = String::new();
    fn write_code_block(arena: &Arena, message: &mut String, cb: PoolPtr) {
        write!(
            message,
            "\tat {}",
            arena
                .get_code_block(ValRef(cb))
                .name
                .as_deref()
                .unwrap_or("[anonymous]")
        )
        .unwrap();
    }
    write_code_block(arena, &mut message, vm.current_code_block);
    for ReturnPoint { code_block, .. } in vm.return_stack.iter() {
        write_code_block(arena, &mut message, *code_block);
    }
    let msg_r = arena.insert_rooted(Value::String(RefCell::new(message)));
    error.map_error(|e| arena.insert_rooted(Value::Pair(Cell::new(e.vr()), Cell::new(msg_r.vr()))))
}

fn handle_error(arena: &Arena, vm: &mut Vm, e: Error) -> Result<RootPtr, RootPtr> {
    let annotated_e = error_stack(arena, &vm, e);
    match annotated_e {
        Error::Abort(v) => Err(v),
        Error::Raise(v) => {
            let handler = arena.get_activation_frame(vm.global_env).borrow().values[0];
            match arena.get(handler) {
                Value::Boolean(false) => Err(v),
                Value::Lambda { .. } => {
                    let frame = ActivationFrame {
                        parent: None,
                        values: vec![v.vr()],
                    };
                    vm.fun = handler;
                    vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(frame))));
                    invoke(arena, vm, false).map_err(|e| e.into_value())?;
                    vm.pc += 1;
                    Ok(arena.root(arena.unspecific))
                }
                _ => {
                    Err(arena.insert_rooted(Value::String(RefCell::new("invalid handler".into()))))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Continuation {
    stack: Vec<ValRef>,
    return_stack: Vec<ReturnPoint>,
}

impl heap::Inventory for Continuation {
    fn inventory(&self, v: &mut heap::PtrVec) {
        for obj in self.stack.iter() {
            v.push(obj.0);
        }
    }
}

enum Error {
    Raise(RootPtr),
    Abort(RootPtr),
}

impl Error {
    fn map_error<F>(self, f: F) -> Error
    where
        F: FnOnce(RootPtr) -> RootPtr,
    {
        match self {
            Error::Raise(v) => Error::Raise(f(v)),
            Error::Abort(v) => Error::Abort(f(v)),
        }
    }

    fn into_value(self) -> RootPtr {
        match self {
            Error::Raise(v) => v,
            Error::Abort(v) => v,
        }
    }
}

fn raise_string(arena: &Arena, error: String) -> Error {
    Error::Raise(arena.insert_rooted(Value::String(RefCell::new(error))))
}
