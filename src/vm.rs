use arena::Arena;

pub enum Instruction {
    Constant(usize),
    Finish,
}

struct Vm {
    value: usize,
    code: Vec<Instruction>,
    pc: usize,
}

pub fn run(arena: &mut Arena, code: Vec<Instruction>) -> usize {
    let mut vm = Vm {
        value: arena.unspecific,
        code,
        pc: 0,
    };
    loop {
        match vm.code[vm.pc] {
            Instruction::Constant(v) => vm.value = v,
            Instruction::Finish => break,
        }
        vm.pc += 1;
    }
    vm.value
}
