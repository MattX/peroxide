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

extern crate core;
extern crate rustyline;

use std::cell::RefCell;
use std::rc::Rc;

use arena::Arena;
use ast::{largest_toplevel_reference, SyntaxElement};
use environment::{ActivationFrame, Environment, RcEnv};
use value::Value;
use vm::Instruction;

pub mod arena;
pub mod ast;
pub mod compile;
pub mod environment;
pub mod gc;
pub mod lex;
pub mod primitives;
pub mod read;
pub mod repl;
pub mod util;
pub mod value;
pub mod vm;

/// Structure holding the global state of the interpreter.
pub struct VmState {
    pub global_environment: RcEnv,
    pub global_frame: usize,
    pub code: Vec<Instruction>,
}

impl VmState {
    pub fn new(arena: &Arena) -> Self {
        let mut global_environment = Rc::new(RefCell::new(Environment::new(None)));
        let global_frame = arena.insert(Value::ActivationFrame(RefCell::new(ActivationFrame {
            parent: None,
            values: vec![],
        })));
        primitives::register_primitives(&arena, &mut global_environment, global_frame);

        VmState {
            global_environment,
            global_frame,
            code: vec![],
        }
    }
}

/// High-level interface to parse, compile, and run a value that's been read.
pub fn parse_compile_run(arena: &Arena, state: &mut VmState, read: usize) -> Result<usize, String> {
    let cloned_env = state.global_environment.clone();
    let syntax_tree = ast::parse(arena, state, &cloned_env, read, true)
        .map_err(|e| format!("Syntax error: {}", e))?;
    println!(" => {:?}", syntax_tree);
    compile_run(arena, state, &syntax_tree)
}

pub fn compile_run(
    arena: &Arena,
    state: &mut VmState,
    syntax_tree: &SyntaxElement,
) -> Result<usize, String> {
    if let Some(n) = largest_toplevel_reference(&syntax_tree) {
        arena
            .get_activation_frame(state.global_frame)
            .borrow_mut()
            .ensure_index(arena, n);
    }
    let start_pc = state.code.len();
    compile::compile(
        &syntax_tree,
        &mut state.code,
        &state.global_environment,
        false,
        true,
    )
    .map_err(|e| format!("Compilation error: {}", e))?;
    state.code.push(Instruction::Finish);
    println!(" => {:?}", &state.code[start_pc..state.code.len()]);
    vm::run(arena, &state.code, start_pc, state.global_frame)
        .map_err(|e| format!("Runtime error: {}", e))
}