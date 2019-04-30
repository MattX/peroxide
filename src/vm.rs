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
    NoOp,
    Finish,
}

#[derive(Debug)]
struct Vm {
    value: usize,
    code: Vec<Instruction>,
    pc: usize,
    stack: Vec<usize>,
    env: usize,
}

pub fn run(arena: &mut Arena, code: Vec<Instruction>) -> Result<usize, String> {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc: 0,
        stack: Vec::new(),
        env: arena.intern(Value::ActivationFrame(RefCell::new(ActivationFrame {
            parent: None,
            values: vec![],
        }))),
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
                    if actual != arity {
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
                vm.pc = vm.stack.pop().expect("Returning with no values on stack.");
            }
            Instruction::CreateClosure(offset) => {
                vm.value = arena.intern(Lambda {
                    name: "".into(),
                    code: vm.pc + offset,
                    environment: vm.env,
                })
            }
            Instruction::NoOp => return Err("NoOp encountered.".into()),
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    Ok(vm.value)
}
