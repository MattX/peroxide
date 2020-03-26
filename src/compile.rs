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
use vm::{Code, Instruction};
use heap::{RootPtr, Inventory, PtrVec, PoolPtr};
use environment::RcEnv;

/// A unit of code corresponding to a function.
#[derive(Debug, PartialEq, Clone)]
pub struct CodeBlock {
    pub name: Option<String>,
    pub arity: usize,
    pub has_rest: bool,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<PoolPtr>,
    pub environment: RcEnv,
}

impl Inventory for CodeBlock {
    fn inventory(&self, v: &mut PtrVec) {
        for &c in self.constants.iter() {
            v.push(c);
        }
    }
}

impl CodeBlock {
    pub fn new(name: Option<String>, arity: usize, has_rest: bool, environment: &RcEnv) -> Self {
        CodeBlock {
            name,
            arity,
            has_rest,
            instructions: vec![],
            constants: vec![],
            environment: environment.clone(),
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

    pub fn push_constant(&mut self, c: PoolPtr) -> usize {
        self.constants.push(c);
        self.constants.len() - 1
    }
}
pub fn compile(
    tree: &SyntaxElement,
    code: &mut CodeBlock,
    tail: bool,
    toplevel: bool,
) -> Result<usize, String> {
    if tail && toplevel {
        panic!("toplevel expression is not in tail position")
    }
    let initial_len = code.code_size();
    match tree {
        SyntaxElement::Quote(q) => {
            let idx = code.push_constant(q.quoted.pp());
            code.push(Instruction::Constant(idx));
        }
        SyntaxElement::If(i) => {
            compile(&i.cond, code, false, false)?;
            let cond_jump = code.code_size();
            code.push(Instruction::NoOp); // Is rewritten as a conditional jump below
            compile(&i.t, code, tail, false)?;
            let mut true_end = code.code_size();
            if let Some(ref f) = i.f {
                code.push(Instruction::NoOp);
                true_end += 1;
                compile(f, code, tail, false)?;
                let jump_offset = code.code_size() - true_end;
                code.replace(true_end - 1, Instruction::Jump(jump_offset));
            }
            code.replace(cond_jump, Instruction::JumpFalse(true_end - cond_jump - 1));
        }
        SyntaxElement::Begin(b) => {
            compile_sequence(&b.expressions, code, tail)?;
        }
        SyntaxElement::Set(s) => {
            compile(&s.value, code, false, false)?;
            code.push(make_set_instruction(s.altitude, s.depth, s.index));
        }
        SyntaxElement::Reference(r) => {
            code.push(make_get_instruction(r.altitude, r.depth, r.index));
        }
        SyntaxElement::Lambda(l) => {
            code.push(Instruction::CreateClosure(1));
            let skip_pos = code.code_size();
            code.push(Instruction::NoOp); // Will be replaced with over function code
            code.push(Instruction::CheckArity {
                arity: l.arity,
                dotted: l.dotted,
            });
            if l.dotted {
                code.push(Instruction::PackFrame(l.arity));
            }
            code.push(Instruction::ExtendFrame(l.defines.len()));
            code.push(Instruction::ExtendEnv);
            //code.push_env(&l.env);
            //code.push_lambda(&l.name.clone().unwrap_or_else(|| "[anonymous]".into()));

            if !l.defines.is_empty() {
                compile_sequence(&l.defines, code, false)?;
            }
            compile_sequence(&l.expressions, code, true)?;

            //code.pop_lambda();
            //code.pop_env();
            code.push(Instruction::Return);
            let jump_offset = code.code_size() - skip_pos - 1;
            code.replace(skip_pos, Instruction::Jump(jump_offset));
        }
        SyntaxElement::Application(a) => {
            compile(&a.function, code, false, false)?;
            code.push(Instruction::PushValue);
            for instr in a.args.iter() {
                compile(instr, code, false, false)?;
                code.push(Instruction::PushValue);
            }
            code.push(Instruction::CreateFrame(a.args.len()));
            code.push(Instruction::PopFunction);
            if !tail {
                code.push(Instruction::PreserveEnv);
            }
            code.push(Instruction::FunctionInvoke { tail });
            if !tail {
                code.push(Instruction::RestoreEnv);
            }
        }
    }
    Ok(code.code_size() - initial_len)
}

fn compile_sequence(
    expressions: &[SyntaxElement],
    code: &mut CodeBlock,
    tail: bool,
) -> Result<usize, String> {
    let initial_len = code.code_size();
    for instr in expressions[..expressions.len() - 1].iter() {
        compile(instr, code, false, false)?;
    }
    compile(
        // This should have been caught at the syntax step.
        expressions.last().expect("empty sequence"),
        code,
        tail,
        false,
    )?;
    Ok(code.code_size() - initial_len)
}

fn make_get_instruction(altitude: usize, depth: usize, index: usize) -> Instruction {
    match (altitude, false) {
        (0, true) => Instruction::GlobalArgumentGet { index },
        (0, false) => Instruction::CheckedGlobalArgumentGet { index },
        (_, true) => Instruction::LocalArgumentGet { depth, index },
        (_, false) => Instruction::CheckedLocalArgumentGet { depth, index },
    }
}

fn make_set_instruction(altitude: usize, depth: usize, index: usize) -> Instruction {
    match altitude {
        0 => Instruction::GlobalArgumentSet { index },
        _ => Instruction::DeepArgumentSet { depth, index },
    }
}
