use ast::SyntaxElement;
use vm::Instruction;

pub fn compile(tree: &SyntaxElement, to: &mut Vec<Instruction>) -> Result<(), String> {
    match tree {
        SyntaxElement::Quote(q) => {
            to.push(Instruction::Constant(q.quoted));
            Ok(())
        }
        SyntaxElement::If(i) => {
            compile(&i.cond, to)?;
            let conditional_jump_idx = to.len();
            to.push(Instruction::NoOp); // Will later be rewritten as a conditional jump
            compile(&i.t, to)?;
            let mut true_end = to.len();
            if let Some(ref f) = i.f {
                to.push(Instruction::NoOp);
                true_end += 1;
                compile(f, to)?;
                to[true_end - 1] = Instruction::Jump(to.len() - true_end);
            }
            to[conditional_jump_idx] = Instruction::JumpFalse(true_end - conditional_jump_idx - 1);
            Ok(())
        }
        _ => Err(format!("Can't compile {:?}.", tree)),
    }
}
