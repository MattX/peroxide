use ast::SyntaxElement;
use environment::Environment;
use vm::Instruction;

pub fn compile(
    tree: &SyntaxElement,
    to: &mut Vec<Instruction>,
    env: &mut Environment,
    tail: bool,
) -> Result<(), String> {
    match tree {
        SyntaxElement::Quote(q) => {
            to.push(Instruction::Constant(q.quoted));
            Ok(())
        }
        SyntaxElement::If(i) => {
            compile(&i.cond, to, env, false)?;
            let conditional_jump_idx = to.len();
            to.push(Instruction::NoOp); // Will later be rewritten as a conditional jump
            compile(&i.t, to, env, tail)?;
            let mut true_end = to.len();
            if let Some(ref f) = i.f {
                to.push(Instruction::NoOp);
                true_end += 1;
                compile(f, to, env, tail)?;
                to[true_end - 1] = Instruction::Jump(to.len() - true_end);
            }
            to[conditional_jump_idx] = Instruction::JumpFalse(true_end - conditional_jump_idx - 1);
            Ok(())
        }
        SyntaxElement::Begin(b) => {
            for instr in b.expressions[..b.expressions.len() - 1].iter() {
                compile(instr, to, env, false);
            }
            compile(
                b.expressions.last().expect("Begin somehow has no body."),
                to,
                env,
                tail,
            );
            Ok(())
        }
        SyntaxElement::Set(s) => {
            if let Some((depth, index)) = env.get(&s.variable) {
                compile(&s.value, to, env, false)?;
                to.push(Instruction::DeepArgumentSet { depth, index });
                Ok(())
            } else {
                Err(format!("Undefined value {}.", &s.variable))
            }
        }
        SyntaxElement::Lambda(l) => Err("Can't compile lambdas".into()),
        _ => Err(format!("Can't compile {:?}.", tree)),
    }
}
