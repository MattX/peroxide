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

use ast::SyntaxElement;
use environment::RcEnv;
use vm::Instruction;

pub fn compile(
    tree: &SyntaxElement,
    to: &mut Vec<Instruction>,
    env: &RcEnv,
    tail: bool,
    toplevel: bool,
) -> Result<usize, String> {
    if tail && toplevel {
        panic!("Toplevel expression is not in tail position")
    }
    let initial_len = to.len();
    match tree {
        SyntaxElement::Quote(q) => {
            to.push(Instruction::Constant(q.quoted));
        }
        SyntaxElement::If(i) => {
            compile(&i.cond, to, env, false, false)?;
            let conditional_jump_idx = to.len();
            to.push(Instruction::NoOp); // Is rewritten as a conditional jump below
            compile(&i.t, to, env, tail, false)?;
            let mut true_end = to.len();
            if let Some(ref f) = i.f {
                to.push(Instruction::NoOp);
                true_end += 1;
                compile(f, to, env, tail, false)?;
                to[true_end - 1] = Instruction::Jump(to.len() - true_end);
            }
            to[conditional_jump_idx] = Instruction::JumpFalse(true_end - conditional_jump_idx - 1);
        }
        SyntaxElement::Begin(b) => {
            compile_sequence(&b.expressions, to, env, tail)?;
        }
        SyntaxElement::Set(s) => {
            compile(&s.value, to, env, false, false)?;
            to.push(make_set_instruction(env, s.altitude, s.index));
        }
        SyntaxElement::Reference(r) => {
            to.push(make_get_instruction(env, r.altitude, r.index));
        }
        SyntaxElement::Lambda(l) => {
            to.push(Instruction::CreateClosure(1));
            let skip_pos = to.len();
            to.push(Instruction::NoOp); // Will be replaced with over function code
            to.push(Instruction::CheckArity {
                arity: l.arity,
                dotted: l.dotted,
            });
            if l.dotted {
                to.push(Instruction::PackFrame(l.arity));
            }
            to.push(Instruction::ExtendFrame(l.defines.len()));
            to.push(Instruction::ExtendEnv);

            if !l.defines.is_empty() {
                compile_sequence(&l.defines, to, &l.env, false)?;
            }
            compile_sequence(&l.expressions, to, &l.env, true)?;
            to.push(Instruction::Return);
            to[skip_pos] = Instruction::Jump(to.len() - skip_pos - 1);
        }
        SyntaxElement::Application(a) => {
            compile(&a.function, to, env, false, false)?;
            to.push(Instruction::PushValue);
            for instr in a.args.iter() {
                compile(instr, to, env, false, false)?;
                to.push(Instruction::PushValue);
            }
            to.push(Instruction::CreateFrame(a.args.len()));
            to.push(Instruction::PopFunction);
            if !tail {
                to.push(Instruction::PreserveEnv);
            }
            to.push(Instruction::FunctionInvoke { tail });
            if !tail {
                to.push(Instruction::RestoreEnv);
            }
        }
    }
    Ok(to.len() - initial_len)
}

fn compile_sequence(
    expressions: &[SyntaxElement],
    to: &mut Vec<Instruction>,
    env: &RcEnv,
    tail: bool,
) -> Result<usize, String> {
    let initial_len = to.len();
    for instr in expressions[..expressions.len() - 1].iter() {
        compile(instr, to, env, false, false)?;
    }
    compile(
        // This should have been caught at the syntax step.
        expressions.last().expect("Empty sequence."),
        to,
        env,
        tail,
        false,
    )?;
    Ok(to.len() - initial_len)
}

fn make_get_instruction(env: &RcEnv, altitude: usize, index: usize) -> Instruction {
    let depth = env.borrow().depth(altitude);
    match (altitude, false) {
        (0, true) => Instruction::GlobalArgumentGet { index },
        (0, false) => Instruction::CheckedGlobalArgumentGet { index },
        (_, true) => Instruction::LocalArgumentGet { depth, index },
        (_, false) => Instruction::CheckedLocalArgumentGet { depth, index },
    }
}

fn make_set_instruction(env: &RcEnv, altitude: usize, index: usize) -> Instruction {
    let depth = env.borrow().depth(altitude);
    match altitude {
        0 => Instruction::GlobalArgumentSet { index },
        _ => Instruction::DeepArgumentSet { depth, index },
    }
}
