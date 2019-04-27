use arena::Arena;

#[derive(Debug)]
pub enum Instruction {
    Constant(usize),
    JumpFalse(usize),
    Jump(usize),
    NoOp,
    Finish,
}

#[derive(Debug)]
struct Vm {
    value: usize,
    code: Vec<Instruction>,
    pc: usize,
}

pub fn run(arena: &mut Arena, code: Vec<Instruction>) -> Result<usize, String> {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc: 0,
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
            Instruction::NoOp => return Err("NoOp encountered.".into()),
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    Ok(vm.value)
}
