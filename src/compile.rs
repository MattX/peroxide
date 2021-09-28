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

use std::cell::RefCell;

use arena::Arena;
use ast::{Lambda, LocatedSyntaxElement, SyntaxElement};
use environment::RcEnv;
use heap::{Inventory, PoolPtr, PtrVec};
use value::Value;
use vm::Instruction;

/// A unit of code corresponding to a function.
#[derive(Debug, PartialEq, Clone)]
pub struct CodeBlock {
    pub name: Option<String>,
    pub arity: usize,
    pub dotted: bool,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<PoolPtr>,
    pub code_blocks: Vec<PoolPtr>,
    pub environment: RcEnv,
}

impl Inventory for CodeBlock {
    fn inventory(&self, v: &mut PtrVec) {
        for &c in self.constants.iter() {
            v.push(c);
        }
        for &c in self.code_blocks.iter() {
            v.push(c);
        }
    }
}

impl CodeBlock {
    pub fn new(name: Option<String>, arity: usize, dotted: bool, environment: RcEnv) -> Self {
        CodeBlock {
            name,
            arity,
            dotted,
            instructions: vec![],
            constants: vec![],
            code_blocks: vec![],
            environment,
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

pub fn compile_toplevel(arena: &Arena, tree: &SyntaxElement, environment: RcEnv) -> PoolPtr {
    let mut code_block = CodeBlock::new(Some("[toplevel]".into()), 0, false, environment);

    // rooted_vec is a bit of a hack to avoid accidentally GCing code blocks.
    // Why is this needed? CodeBlock objects are immutable once inserted, so we'll have to insert
    // the toplevel one at the very end of the compilation procedure. This means that any sub-
    // CodeBlocks aren't rooted even if they are added to the toplevel CodeBlock's code_blocks
    // array. To alleviate this, we create this additional mutable vector to which we can add
    // items in progress.
    let rooted_vec = arena.insert_rooted(Value::Vector(RefCell::new(vec![])));

    compile(arena, tree, &mut code_block, false, rooted_vec.pp());
    code_block.push(Instruction::Finish);
    arena.insert(Value::CodeBlock(Box::new(code_block)))
}

pub fn compile(arena: &Arena, tree: &SyntaxElement, code: &mut CodeBlock, tail: bool, rv: PoolPtr) {
    match tree {
        SyntaxElement::Quote(q) => {
            let idx = code.push_constant(q.quoted.pp());
            code.push(Instruction::Constant(idx));
        }
        SyntaxElement::If(i) => {
            compile(arena, &i.cond.element, code, false, rv);
            let cond_jump = code.code_size();
            code.push(Instruction::NoOp); // Is rewritten as a conditional jump below
            compile(arena, &i.t.element, code, tail, rv);
            let mut true_end = code.code_size();
            if let Some(ref f) = i.f {
                code.push(Instruction::NoOp);
                true_end += 1;
                compile(arena, &f.element, code, tail, rv);
                let jump_offset = code.code_size() - true_end;
                code.replace(true_end - 1, Instruction::Jump(jump_offset));
            }
            code.replace(cond_jump, Instruction::JumpFalse(true_end - cond_jump - 1));
        }
        SyntaxElement::Begin(b) => {
            compile_sequence(arena, &b.expressions, code, tail, rv);
        }
        SyntaxElement::Set(s) => {
            compile(arena, &s.value.element, code, false, rv);
            code.push(make_set_instruction(s.altitude, s.depth, s.index));
        }
        SyntaxElement::Reference(r) => {
            code.push(make_get_instruction(r.altitude, r.depth, r.index));
        }
        SyntaxElement::Lambda(l) => {
            code.code_blocks.push(compile_lambda(arena, l, rv));
            code.push(Instruction::CreateClosure(code.code_blocks.len() - 1));
        }
        SyntaxElement::Application(a) => {
            compile(arena, &a.function.element, code, false, rv);
            code.push(Instruction::PushValue);
            for instr in a.args.iter() {
                compile(arena, &instr.element, code, false, rv);
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
    };
}

fn compile_sequence(
    arena: &Arena,
    expressions: &[LocatedSyntaxElement],
    code: &mut CodeBlock,
    tail: bool,
    rv: PoolPtr,
) {
    for instr in expressions[..expressions.len() - 1].iter() {
        compile(arena, &instr.element, code, false, rv);
    }
    compile(
        arena,
        // This should have been caught at the syntax step.
        &expressions.last().expect("empty sequence").element,
        code,
        tail,
        rv,
    );
}

fn compile_lambda(arena: &Arena, l: &Lambda, rv: PoolPtr) -> PoolPtr {
    let mut code = CodeBlock::new(l.name.clone(), l.arity, l.dotted, l.env.clone());
    // See `compile_toplevel` for an explanation of rooted_vec
    let rooted_vec = arena.insert_rooted(Value::Vector(RefCell::new(vec![])));

    code.push(Instruction::CheckArity {
        arity: l.arity,
        dotted: l.dotted,
    });
    if l.dotted {
        code.push(Instruction::PackFrame(l.arity));
    }
    code.push(Instruction::ExtendFrame(l.defines.len()));
    code.push(Instruction::ExtendEnv);

    if !l.defines.is_empty() {
        compile_sequence(arena, &l.defines, &mut code, false, rooted_vec.pp());
    }
    compile_sequence(arena, &l.expressions, &mut code, true, rooted_vec.pp());

    code.push(Instruction::Return);

    let code_block_ptr = arena.insert(Value::CodeBlock(Box::new(code)));
    rv.try_get_vector()
        .unwrap()
        .borrow_mut()
        .push(code_block_ptr);
    // println!("{:?}", code_block_ptr.pretty_print());
    code_block_ptr
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
