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
use environment::{ActivationFrame, RcEnv};
use heap;
use heap::{Inventory, PoolPtr, PtrVec, RootPtr};
use primitives::PrimitiveImplementation;
use value::{list_from_vec, pretty_print, vec_from_list, Value};

static MAX_RECURSION_DEPTH: usize = 1000;

#[derive(Debug, Clone, Copy)]
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
    constants: Vec<RootPtr>,
}

impl Code {
    pub fn new(global_environment: &RcEnv) -> Self {
        Code {
            instructions: vec![],
            env_map: vec![(0, global_environment.clone())],
            env_stack: vec![global_environment.clone()],
            lambda_map: vec![(0, None)],
            lambda_stack: vec![],
            constants: vec![],
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

    pub fn push_constant(&mut self, c: RootPtr) -> usize {
        self.constants.push(c);
        self.constants.len() - 1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vm {
    value: ValRef,
    pc: usize,
    return_stack: Vec<usize>,
    stack: Vec<ValRef>,
    global_env: ValRef,
    env: ValRef,
    fun: ValRef,
}

impl Vm {
    fn set_value(&mut self, v: ValRef) {
        #[cfg(debug_assertions)] {
            debug_assert!(v.0.ok());
        }
        self.value = v;
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

    fn get_value(&self) -> ValRef {
        match self {
            Error::Raise(v) => v.vr(),
            Error::Abort(v) => v.vr(),
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

// TODO rename env around here to frame
pub fn run(
    arena: &Arena,
    code: &mut Code,
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
    };
    // println!("rooting VM");
    arena.root_vm(&vm);
    let res = loop {
        match run_one(arena, &mut vm, code) {
            Ok(true) => break Ok(arena.root(vm.value)),
            Ok(_) => (),
            Err(e) => break handle_error(arena, &mut vm, code, e),
        }
    };
    // println!("unrooting VM");
    arena.unroot_vm();
    res
}

fn handle_error(arena: &Arena, vm: &mut Vm, code: &Code, e: Error) -> Result<RootPtr, RootPtr> {
    let annotated_e = error_stack(arena, &vm, code, e);
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

fn run_one(arena: &Arena, vm: &mut Vm, code: &mut Code) -> Result<bool, Error> {
    // println!("running {:?}", code.instructions[vm.pc]);
    match code.instructions[vm.pc] {
        Instruction::Constant(v) => vm.set_value(code.constants[v].vr()),
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
            vm.set_value(get_activation_frame(arena, vm.global_env)
                .borrow()
                .get(arena, 0, index));
        }
        Instruction::CheckedGlobalArgumentGet { index } => {
            vm.set_value(arena
                .get_activation_frame(vm.global_env)
                .borrow()
                .get(arena, 0, index));
            if vm.value == arena.undefined {
                return Err(raise_string(
                    arena,
                    format!(
                        "Variable used before definition: {}",
                        resolve_variable(code, vm.pc, 0, index)
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
            vm.set_value(get_activation_frame(arena, vm.env)
                .borrow()
                .get(arena, depth, index));
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
                        resolve_variable(code, vm.pc, current_depth - depth, index)
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
            vm.pc = vm
                .return_stack
                .pop()
                .expect("Returning with no values on return stack.");
        }
        Instruction::CreateClosure(offset) => {
            vm.set_value(arena.insert(Value::Lambda {
                code: vm.pc + offset,
                environment: vm.env,
            }))
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
                        format!("cannot apply non-function: {}", pretty_print(arena, fun_r)),
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
            let stack_len = vm.stack.len();
            for i in (0..size).rev() {
                frame.values[i] = *vm.stack.get(stack_len - size + i).expect("too few values on stack.");
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
        Value::Lambda {
            code, environment, ..
        } => {
            if !tail {
                if vm.return_stack.len() > MAX_RECURSION_DEPTH {
                    return Err(Error::Abort(arena.insert_rooted(Value::String(
                        RefCell::new("Maximum recursion depth exceeded".into()),
                    ))));
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
                vm.set_value(i(arena, &values)
                    .map_err(|e| raise_string(arena, format!("In {:?}: {}", p, e)))?);
            }
            PrimitiveImplementation::Apply => apply(arena, vm, tail)?,
            PrimitiveImplementation::CallCC => call_cc(arena, vm)?,
            PrimitiveImplementation::Abort => return Err(raise(arena, vm, true)),
            PrimitiveImplementation::Raise => return Err(raise(arena, vm, false)),
            _ => return Err(raise_string(arena, format!("Unimplemented: {}", p.name))),
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
            vm.pc = vm
                .return_stack
                .pop()
                .expect("Popping continuation with no return address");
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
        return Err(raise_string(arena, "apply: too few arguments.".into()));
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
    vm.return_stack.push(vm.pc);
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

fn resolve_variable(code: &Code, pc: usize, altitude: usize, index: usize) -> String {
    let env = code.find_env(pc).borrow();
    env.get_name(altitude, index)
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

fn error_stack(arena: &Arena, vm: &Vm, code: &Code, error: Error) -> Error {
    let mut message = String::new();
    let positions = std::iter::once(&vm.pc).chain(vm.return_stack.iter().rev());
    for ret in positions {
        let name = code
            .find_lambda(*ret)
            .clone()
            .unwrap_or_else(|| "<toplevel>".into());
        write!(message, "\n\tat {}", name).unwrap();
    }
    let msg_r = arena.insert_rooted(Value::String(RefCell::new(message)));
    error.map_error(|e| arena.insert_rooted(Value::Pair(Cell::new(e.vr()), Cell::new(msg_r.vr()))))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Continuation {
    stack: Vec<ValRef>,
    return_stack: Vec<usize>,
}

impl heap::Inventory for Continuation {
    fn inventory(&self, v: &mut heap::PtrVec) {
        for obj in self.stack.iter() {
            v.push(obj.0);
        }
    }
}
