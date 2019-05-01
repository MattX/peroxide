use arena::Arena;
use core::borrow::{Borrow, BorrowMut};
use environment::ActivationFrame;
use std::cell::RefCell;
use std::rc::Rc;
use value::Value;
use value::Value::Lambda;

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
    PreserveEnv,
    RestoreEnv,
    PushValue,
    PopFunction,
    FunctionInvoke,
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
    env: usize,
    fun: usize,
}

pub fn run(arena: &mut Arena, code: &[Instruction], pc: usize) -> Result<usize, String> {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc,
        return_stack: Vec::new(),
        stack: Vec::new(),
        env: arena.intern(Value::ActivationFrame(RefCell::new(ActivationFrame {
            parent: None,
            values: vec![],
        }))),
        fun: 0,
    };
    loop {
        match vm.code[vm.pc] {
            Instruction::Constant(v) => vm.value = v,
            Instruction::JumpFalse(offset) => {
                if !arena.value_ref(vm.value).truthy() {
                    vm.pc += offset;
                }
            }
            Instruction::Jump(offset) => vm.pc += offset,
            Instruction::DeepArgumentSet { depth, index } => {
                if let Value::ActivationFrame(af) = arena.value_ref(vm.env) {
                    af.borrow_mut().set(arena, depth, index, vm.value)
                } else {
                    panic!("Environment is not an activation frame.");
                }
            }
            Instruction::DeepArgumentGet { depth, index } => {
                if let Value::ActivationFrame(af) = arena.value_ref(vm.env) {
                    vm.value = af.borrow().get(arena, depth, index)
                } else {
                    panic!("Environment is not an activation frame.");
                }
            }
            Instruction::CheckArity { arity, dotted } => {
                if let Value::ActivationFrame(af) = arena.value_ref(vm.value) {
                    let actual = af.borrow().values.len();
                    if actual != arity + 1 {
                        return Err(format!("Expected {} arguments, got {}.", arity, actual));
                    }
                } else {
                    panic!("Checking arity: value is not an activation frame.");
                }
            }
            Instruction::ExtendEnv => {
                if let Value::ActivationFrame(af) = arena.value_ref(vm.value) {
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
                vm.value = arena.intern(Lambda {
                    name: "".into(),
                    code: vm.pc + offset,
                    environment: vm.env,
                })
            }
            Instruction::PreserveEnv => {
                vm.stack.push(vm.env);
            }
            Instruction::RestoreEnv => {
                let env_r = vm
                    .stack
                    .pop()
                    .expect("Restoring env with no values on stack.");
                if let Value::ActivationFrame(af) = arena.value_ref(env_r) {
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
                match arena.value_ref(fun_r) {
                    Value::Lambda { .. } | Value::Primitive(_) => vm.fun = fun_r,
                    _ => panic!("Popping non-function."),
                }
            }
            Instruction::FunctionInvoke => {
                // TODO handle non-lambda here, and TR stuff
                match arena.value_ref(vm.fun) {
                    Value::Lambda {
                        code, environment, ..
                    } => {
                        vm.return_stack.push(vm.pc);
                        vm.env = *environment;
                        vm.pc = *code;
                    }
                    _ => return Err("Cannot invoke non-lambdas yet".into()),
                }
            }
            Instruction::CreateFrame(size) => {
                let mut frame = ActivationFrame {
                    parent: None,
                    values: vec![0; size + 1],
                };
                for i in (0..size).rev() {
                    frame.values[i] = vm.stack.pop().expect("Too few values on stack.");
                }
                vm.value = arena.intern(Value::ActivationFrame(RefCell::new(frame)));
            }
            Instruction::NoOp => return Err("NoOp encountered.".into()),
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    Ok(vm.value)
}
