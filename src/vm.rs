// Copyright 2018-2020 Matthieu Felix
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
use std::sync::atomic::Ordering::Relaxed;

use arena::Arena;
use environment::ActivationFrame;
use heap::{Inventory, PoolPtr, PtrVec, RootPtr};
use primitives::PrimitiveImplementation;
use value::{list_from_vec, Value};
use {heap, Interpreter, INPUT_PORT_INDEX, OUTPUT_PORT_INDEX};

static MAX_RECURSION_DEPTH: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    /// Loads a constant to the VM's value register. The attached usize is
    /// an index into the current code block's `constants` array.
    Constant(usize),

    /// Jump to the given location in the current code block if the value in
    /// the VM's value register is falsy (i.e. is anything other than `#f`).
    JumpFalse(usize),

    /// Jumps to the given location in the current code block unconditionally.
    Jump(usize),

    /// Sets the value in the global frame at the given index to the value in
    /// the VM's value register.
    GlobalArgumentSet { index: usize },

    /// Loads the value in the global frame at the given index into the VM's
    /// value register.
    GlobalArgumentGet { index: usize },

    /// Same as GlobalArgumentGet, but validates that the value is not undefined.
    CheckedGlobalArgumentGet { index: usize },

    /// Sets the value at the given index in the frame at the given depth
    /// relative to the VM's env register to the value in the VM's value
    /// register.
    DeepArgumentSet { depth: usize, index: usize },

    /// Loads the value at the given index in the frame at the given depth
    /// relative to the VM's env register into the VM's value register
    LocalArgumentGet { depth: usize, index: usize },

    /// Same as LocalArgumentGet, but validates that the value is not undefined.
    CheckedLocalArgumentGet { depth: usize, index: usize },

    /// Checks that the activation frame in the VM's env register has the correct
    /// length (or at least the correct length if dotted)
    // TODO we might be able to just get the arguments from the local code block.
    CheckArity { arity: usize, dotted: bool },

    /// Takes the frame currently in the VM's value register, and sets its
    /// parent to be the frame in the VM's env register. Then, set the VM's
    /// env register to the frame that was just modified. Panics if the value
    /// register does not contain a frame.
    ExtendEnv,

    /// Pops a code block and a program counter from the return stack, and set
    /// the VM's code_block and pc registers to these values.
    Return,

    /// Create a Lambda object whose definition frame is the VM's env register.
    /// The attached usize is an index into the current code block's `code_blocks`
    /// array and determines the code block containing code for the Lambda to
    /// create.
    CreateClosure(usize),

    /// Loads the activation frame in the VM's value register and collect any elements
    /// with indices greater than arity into a list, that is then placed at the end
    /// of the activation frame.
    PackFrame(usize),

    /// Adds slots to the activation frame in the VM's value register.
    ExtendFrame(usize),

    /// Pushes the VM's env register onto the stack.
    PreserveEnv,

    /// Pops the top element of the stack, which must be an activation frame,
    /// and load it into the VM's env register.
    RestoreEnv,

    /// Pushes the VM's value register onto the stack.
    PushValue,

    /// Pops the top element of the stack, which must be a Lambda, and sets it
    /// as the VM's fun register.
    PopFunction,

    /// Invokes the function in the VM's fun register. See invoke() for more details.
    FunctionInvoke { tail: bool },

    /// Create a new activation frame with the attached number of items in it by
    /// popping that number of values from the stack. The topmost item from the stack
    /// goes in the last location in the frame.
    CreateFrame(usize),

    /// Placeholder opcode used during compilation.
    NoOp,

    /// Signals that the toplevel code is done executing. The VM driver loop
    /// will return when this happen.
    Finish,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnPoint {
    pub code_block: PoolPtr,

    /// Holds the index of the last executed instruction
    pub pc: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vm {
    value: PoolPtr,
    pc: usize,
    return_stack: Vec<ReturnPoint>,
    stack: Vec<PoolPtr>,
    global_env: PoolPtr,
    env: PoolPtr,
    fun: PoolPtr,
    root_code_block: PoolPtr,
    current_code_block: PoolPtr,
}

impl Vm {
    fn set_value(&mut self, v: PoolPtr) {
        debug_assert!(v.ok());
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
        v.push(self.value);
        v.push(self.global_env);
        v.push(self.env);
        v.push(self.fun);
        for s in self.stack.iter() {
            v.push(*s);
        }
        v.push(self.root_code_block);
        v.push(self.current_code_block);
        for rp in self.return_stack.iter() {
            v.push(rp.code_block);
        }
    }
}

// TODO rename env around here to frame
pub fn run(code: RootPtr, pc: usize, env: PoolPtr, int: &Interpreter) -> Result<RootPtr, RootPtr> {
    let mut vm = Vm {
        value: int.arena.unspecific,
        pc,
        return_stack: Vec::new(),
        stack: Vec::new(),
        global_env: int.global_frame.pp(),
        env,
        fun: int.arena.unspecific,
        root_code_block: code.pp(),
        current_code_block: code.pp(),
    };
    int.arena.root_vm(&vm);
    let res = loop {
        if int.interruptor.load(Relaxed) {
            int.interruptor.store(false, Relaxed);
            break Err(int
                .arena
                .insert_rooted(Value::String(RefCell::new("interrupted".into()))));
        };
        match run_one_instruction(int, &mut vm) {
            Ok(true) => break Ok(int.arena.root(vm.value)),
            Ok(_) => (),
            Err(e) => break handle_error(int, &mut vm, e),
        }
    };
    int.arena.unroot_vm();
    res
}

fn run_one_instruction(int: &Interpreter, vm: &mut Vm) -> Result<bool, Error> {
    let arena = &int.arena;
    let code = vm.current_code_block.long_lived().get_code_block();
    let instr = code.instructions[vm.pc];
    // println!("running {:?}, pc {}", instr, vm.pc);
    match instr {
        Instruction::Constant(v) => vm.set_value(code.constants[v]),
        Instruction::JumpFalse(offset) => {
            if !vm.value.truthy() {
                vm.pc += offset;
            }
        }
        Instruction::Jump(offset) => vm.pc += offset,
        Instruction::GlobalArgumentSet { index } => {
            vm.global_env
                .long_lived()
                .get_activation_frame()
                .borrow_mut()
                .set(arena, 0, index, vm.value);
            vm.set_value(arena.unspecific);
        }
        Instruction::GlobalArgumentGet { index } => {
            vm.set_value(
                vm.global_env
                    .long_lived()
                    .get_activation_frame()
                    .borrow()
                    .get(arena, 0, index),
            );
        }
        Instruction::CheckedGlobalArgumentGet { index } => {
            vm.set_value(
                vm.global_env
                    .long_lived()
                    .get_activation_frame()
                    .borrow()
                    .get(arena, 0, index),
            );
            if vm.value == arena.undefined {
                return Err(raise_string(
                    arena,
                    format!(
                        "variable used before definition: {}",
                        resolve_variable(vm, 0, index)
                    ),
                ));
            }
        }
        Instruction::DeepArgumentSet { depth, index } => {
            vm.env
                .long_lived()
                .get_activation_frame()
                .borrow_mut()
                .set(arena, depth, index, vm.value);
            vm.set_value(arena.unspecific);
        }
        Instruction::LocalArgumentGet { depth, index } => {
            vm.set_value(
                vm.env
                    .long_lived()
                    .get_activation_frame()
                    .borrow()
                    .get(arena, depth, index),
            );
        }
        Instruction::CheckedLocalArgumentGet { depth, index } => {
            let frame = vm.env.long_lived().get_activation_frame().borrow();
            vm.set_value(frame.get(arena, depth, index));
            if vm.value == arena.undefined {
                let current_depth = frame.depth();
                return Err(raise_string(
                    arena,
                    format!(
                        "variable used before definition: {}",
                        resolve_variable(vm, current_depth - depth, index)
                    ),
                ));
            }
        }
        Instruction::CheckArity { arity, dotted } => {
            let actual_arity = vm
                .value
                .long_lived()
                .get_activation_frame()
                .borrow()
                .values
                .len();
            if dotted && actual_arity < arity {
                return Err(raise_string(
                    arena,
                    format!(
                        "expected at least {} arguments, got {}",
                        arity, actual_arity
                    ),
                ));
            } else if !dotted && actual_arity != arity {
                return Err(raise_string(
                    arena,
                    format!("expected {} arguments, got {}", arity, actual_arity),
                ));
            }
        }
        Instruction::ExtendEnv => {
            vm.value
                .long_lived()
                .get_activation_frame()
                .borrow_mut()
                .parent = Some(vm.env);
            vm.env = vm.value;
        }
        Instruction::Return => {
            let ReturnPoint { code_block, pc } = vm
                .return_stack
                .pop()
                .expect("returning with no values on return stack");
            vm.current_code_block = code_block;
            vm.pc = pc;
        }
        Instruction::CreateClosure(idx) => {
            vm.set_value(arena.insert(Value::Lambda {
                code: code.code_blocks[idx],
                frame: vm.env,
            }));
        }
        Instruction::PackFrame(arity) => {
            let frame = vm.value.long_lived().get_activation_frame();
            let values = frame.borrow_mut().values.clone();
            if values.len() < arity {
                // This is a panic because we should already have checked in a prior CheckArity
                // call.
                panic!(
                    "too few arguments for varargs method, expecting {}, got {}",
                    arity,
                    values.len()
                );
            }
            let listified = list_from_vec(arena, &values[arity..values.len()]);
            frame.borrow_mut().values.resize(arity + 1, arena.undefined);
            frame.borrow_mut().values[arity] = listified;
        }
        Instruction::ExtendFrame(by) => {
            let mut frame = vm.value.long_lived().get_activation_frame().borrow_mut();
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
            if let Value::ActivationFrame(_) = &*env_r {
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
            match &*fun_r {
                Value::Lambda { .. } | Value::Primitive(_) | Value::Continuation(_) => {
                    vm.fun = fun_r
                }
                _ => {
                    return Err(raise_string(
                        arena,
                        format!("cannot apply non-function: {}", fun_r.pretty_print()),
                    ));
                }
            }
        }
        Instruction::FunctionInvoke { tail } => {
            invoke(int, vm, tail)?;
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
    match instr {
        Instruction::FunctionInvoke { .. } => {}
        _ => vm.pc += 1,
    }
    Ok(false)
}

fn invoke(int: &Interpreter, vm: &mut Vm, tail: bool) -> Result<(), Error> {
    let arena = &int.arena;
    match vm.fun.long_lived() {
        Value::Lambda { code, frame } => {
            if !tail {
                if vm.return_stack.len() > MAX_RECURSION_DEPTH {
                    return Err(Error::Abort(arena.insert_rooted(Value::String(
                        RefCell::new("maximum recursion depth exceeded".into()),
                    ))));
                }
                vm.return_stack.push(vm.get_return_point());
            }
            vm.env = *frame;
            vm.current_code_block = *code;
            vm.pc = 0;
        }
        Value::Primitive(p) => {
            match p.implementation {
                PrimitiveImplementation::Simple(i) => {
                    let af = vm.value.long_lived().get_activation_frame();
                    let values = &af.borrow().values;
                    vm.set_value(
                        i(arena, values)
                            .map_err(|e| raise_string(arena, format!("In {:?}: {}", p, e)))?,
                    );
                }
                PrimitiveImplementation::Io(i) => {
                    let af = vm.value.long_lived().get_activation_frame();
                    let values = &af.borrow().values;
                    let global_env = vm.global_env.long_lived().get_activation_frame().borrow();
                    let input_port = global_env.values[INPUT_PORT_INDEX];
                    let output_port = global_env.values[OUTPUT_PORT_INDEX];
                    vm.set_value(
                        i(arena, input_port, output_port, values)
                            .map_err(|e| raise_string(arena, format!("In {:?}: {}", p, e)))?,
                    );
                }
                PrimitiveImplementation::Apply => apply(int, vm, tail)?,
                PrimitiveImplementation::CallCC => call_cc(int, vm)?,
                PrimitiveImplementation::Abort => return Err(raise(arena, vm, true)),
                PrimitiveImplementation::Raise => return Err(raise(arena, vm, false)),
                PrimitiveImplementation::Eval => eval(int, vm)?,
            };
            match p.implementation {
                PrimitiveImplementation::Apply | PrimitiveImplementation::CallCC => {}
                _ => vm.pc += 1,
            };
        }
        Value::Continuation(c) => {
            let af = vm.value.long_lived().get_activation_frame().borrow();
            if af.values.len() != 1 {
                return Err(raise_string(
                    arena,
                    "invoking continuation with more than one argument".into(),
                ));
            }
            vm.stack = c.stack.clone();
            vm.return_stack = c.return_stack.clone();
            let ReturnPoint { code_block, pc } = vm
                .return_stack
                .pop()
                .expect("popping continuation with no return address");
            vm.current_code_block = code_block;
            vm.pc = pc + 1;
            vm.set_value(af.values[0]);
        }
        _ => {
            return Err(raise_string(
                arena,
                format!("cannot invoke non-function: {}", vm.fun.pretty_print()),
            ));
        }
    }
    Ok(())
}

// TODO apply isn't really tail-recursive, and this could be fixed by returning to the
//      trampoline here.
fn apply(int: &Interpreter, vm: &mut Vm, tail: bool) -> Result<(), Error> {
    let arena = &int.arena;
    let af = vm.value.long_lived().get_activation_frame().borrow();
    let n_args = af.values.len();
    if n_args < 2 {
        return Err(raise_string(arena, "apply: too few arguments".into()));
    }
    let mut values = af.values[1..n_args - 1].to_vec();
    let vec = af.values[n_args - 1]
        .list_to_vec()
        .map_err(|e| raise_string(arena, e))?;
    values.extend(vec.into_iter());
    let new_af = ActivationFrame {
        parent: None,
        values,
    };
    vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(new_af))));
    vm.fun = af.values[0];
    invoke(int, vm, tail)
}

fn call_cc(int: &Interpreter, vm: &mut Vm) -> Result<(), Error> {
    let arena = &int.arena;
    vm.return_stack.push(vm.get_return_point());
    let cont = Continuation {
        stack: vm.stack.clone(),
        return_stack: vm.return_stack.clone(),
    };
    let cont_r = arena.insert(Value::Continuation(cont));
    let af = vm.value.long_lived().get_activation_frame().borrow();
    if af.values.len() != 1 {
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
    invoke(int, vm, true)
}

// fn eval(arena: &Arena, vm: &mut Vm, env: &RcEnv) -> Result<(), Error> {
fn eval(int: &Interpreter, vm: &mut Vm) -> Result<(), Error> {
    let arena = &int.arena;
    let af = vm.value.long_lived().get_activation_frame().borrow();
    if af.values.len() != 2 {
        return Err(raise_string(arena, "eval: expected 2 arguments".into()));
    }
    let expr = af.values[0];
    let _env_descriptor = af.values[1]
        .try_get_string()
        .ok_or_else(|| {
            raise_string(
                arena,
                format!("eval: invalid environment descriptor: {}", &*af.values[1]),
            )
        })?
        .borrow()
        .clone();

    let res = int
        .parse_compile_run(arena.root(expr))
        .map_err(|e| raise_string(arena, format!("eval: {}", e)))?;
    vm.set_value(res.pp());
    Ok(())
}

fn resolve_variable(vm: &Vm, altitude: usize, index: usize) -> String {
    let env = &vm.current_code_block.get_code_block().environment;
    env.borrow().get_name(altitude, index)
}

fn raise(arena: &Arena, vm: &Vm, abort: bool) -> Error {
    let af = vm.value.long_lived().get_activation_frame().borrow();
    if af.values.len() != 1 {
        raise_string(arena, "raise: expected a single argument".into())
    } else if abort {
        Error::Abort(arena.root(af.values[0]))
    } else {
        Error::Raise(arena.root(af.values[0]))
    }
}

fn error_stack(arena: &Arena, vm: &Vm, error: Error) -> Error {
    let mut message = String::new();
    fn write_code_block(message: &mut String, cb: PoolPtr) {
        write!(
            message,
            "\tat {}",
            cb.get_code_block().name.as_deref().unwrap_or("[anonymous]")
        )
        .unwrap();
    }
    write_code_block(&mut message, vm.current_code_block);
    for ReturnPoint { code_block, .. } in vm.return_stack.iter() {
        write_code_block(&mut message, *code_block);
    }
    let msg_r = arena.insert_rooted(Value::String(RefCell::new(message)));
    error.map_error(|e| arena.insert_rooted(Value::Pair(Cell::new(e.pp()), Cell::new(msg_r.pp()))))
}

fn handle_error(int: &Interpreter, vm: &mut Vm, e: Error) -> Result<RootPtr, RootPtr> {
    let arena = &int.arena;
    let annotated_e = error_stack(arena, vm, e);
    match annotated_e {
        Error::Abort(v) => Err(v),
        Error::Raise(v) => {
            let handler = vm.global_env.get_activation_frame().borrow().values[0];
            match &*handler {
                Value::Boolean(false) => Err(v),
                Value::Lambda { .. } => {
                    let frame = ActivationFrame {
                        parent: None,
                        values: vec![v.pp()],
                    };
                    vm.fun = handler;
                    vm.set_value(arena.insert(Value::ActivationFrame(RefCell::new(frame))));
                    invoke(int, vm, false).map_err(|e| e.into_value())?;
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
    stack: Vec<PoolPtr>,
    return_stack: Vec<ReturnPoint>,
}

impl heap::Inventory for Continuation {
    fn inventory(&self, v: &mut heap::PtrVec) {
        for obj in self.stack.iter() {
            v.push(*obj);
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
