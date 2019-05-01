use ast::Begin;
use ast::SyntaxElement;
use environment::{Environment, RcEnv};
use std::cell::RefCell;
use std::rc::Rc;
use vm::Instruction;

pub fn compile(
    tree: &SyntaxElement,
    to: &mut Vec<Instruction>,
    env: RcEnv,
    tail: bool,
) -> Result<usize, String> {
    let initial_len = to.len();
    match tree {
        SyntaxElement::Quote(q) => {
            to.push(Instruction::Constant(q.quoted));
        }
        SyntaxElement::If(i) => {
            compile(&i.cond, to, env.clone(), false)?;
            let conditional_jump_idx = to.len();
            to.push(Instruction::NoOp); // Will later be rewritten as a conditional jump
            compile(&i.t, to, env.clone(), tail)?;
            let mut true_end = to.len();
            if let Some(ref f) = i.f {
                to.push(Instruction::NoOp);
                true_end += 1;
                compile(f, to, env.clone(), tail)?;
                to[true_end - 1] = Instruction::Jump(to.len() - true_end);
            }
            to[conditional_jump_idx] = Instruction::JumpFalse(true_end - conditional_jump_idx - 1);
        }
        SyntaxElement::Begin(b) => {
            compile_sequence(&b.expressions, to, env.clone(), tail)?;
        }
        SyntaxElement::Set(s) => {
            if let Some((depth, index)) = env.borrow().get(&s.variable) {
                compile(&s.value, to, env.clone(), false)?;
                to.push(Instruction::DeepArgumentSet { depth, index });
            // TODO push unspecific here
            } else {
                return Err(format!("Undefined value {}.", &s.variable));
            }
        }
        SyntaxElement::Reference(r) => {
            if let Some((depth, index)) = env.borrow().get(&r.variable) {
                to.push(Instruction::DeepArgumentGet { depth, index });
            } else {
                return Err(format!("Undefined value {}.", &r.variable));
            }
        }
        SyntaxElement::Lambda(l) => {
            if l.formals.rest.is_some() {
                return Err("Only fixed functions can be compiled for now.".into());
            }
            to.push(Instruction::CreateClosure(1));
            let skip_pos = to.len();
            to.push(Instruction::NoOp); // Skip over function definition
            to.push(Instruction::CheckArity {
                arity: l.formals.values.len(),
                dotted: false,
            });
            to.push(Instruction::ExtendEnv);
            let formal_name_refs = l
                .formals
                .values
                .iter()
                .map(std::ops::Deref::deref)
                .collect::<Vec<_>>();
            let lambda_env = Rc::new(RefCell::new(Environment::new_initial(
                Some(env.clone()),
                &formal_name_refs,
            )));
            compile_sequence(&l.expressions, to, lambda_env.clone(), true)?;
            to.push(Instruction::Return);
            to[skip_pos] = Instruction::Jump(to.len() - skip_pos - 1);
        }
        SyntaxElement::Define(_) => return Err("Defines are not yet supported".into()),
        SyntaxElement::Application(a) => {
            compile(&a.function, to, env.clone(), false);
            to.push(Instruction::PushValue);
            for instr in a.args.iter() {
                compile(instr, to, env.clone(), false);
                to.push(Instruction::PushValue);
            }
            to.push(Instruction::CreateFrame(a.args.len()));
            to.push(Instruction::PopFunction);
            to.push(Instruction::PreserveEnv);
            to.push(Instruction::FunctionInvoke);
            to.push(Instruction::RestoreEnv);
        }
    }
    Ok(to.len() - initial_len)
}

fn compile_sequence(
    expressions: &[SyntaxElement],
    to: &mut Vec<Instruction>,
    env: RcEnv,
    tail: bool,
) -> Result<usize, String> {
    let initial_len = to.len();
    for instr in expressions[..expressions.len() - 1].iter() {
        compile(instr, to, env.clone(), false)?;
    }
    compile(
        expressions.last().expect("Empty sequence."),
        to,
        env.clone(),
        tail,
    )?;
    Ok(to.len() - initial_len)
}
