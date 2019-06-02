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
use ast::SyntaxElement;
use environment::{ActivationFrame, ActivationFrameInfo, Environment, RcEnv};
use read::read_many;
use std::fs;
use value::{pretty_print, Value};
use vm::{Code, Instruction};

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
    pub code: Code,
}

impl VmState {
    pub fn new(arena: &Arena) -> Self {
        let global_environment = Rc::new(RefCell::new(Environment::new(None)));
        let global_frame = arena.insert(Value::ActivationFrame(RefCell::new(ActivationFrame {
            parent: None,
            values: vec![],
        })));
        primitives::register_primitives(&arena, &global_environment, global_frame);

        VmState {
            global_environment: global_environment.clone(),
            global_frame,
            code: Code::new(&global_environment),
        }
    }
}

pub fn initialize(arena: &Arena, state: &mut VmState, fname: &str) -> Result<(), String> {
    let contents = fs::read_to_string(fname).map_err(|e| e.to_string())?;
    let values = read_many(arena, &contents)?;
    println!("Values: {:?}", values);
    for v in values.iter() {
        // println!("> {}", pretty_print(arena, *v));
        parse_compile_run(arena, state, *v)?;
    }
    Ok(())
}

/// High-level interface to parse, compile, and run a value that's been read.
pub fn parse_compile_run(arena: &Arena, state: &mut VmState, read: usize) -> Result<usize, String> {
    let cloned_env = state.global_environment.clone();
    let global_af_info = Rc::new(RefCell::new(ActivationFrameInfo {
        parent: None,
        altitude: 0,
        entries: arena
            .get_activation_frame(state.global_frame)
            .borrow()
            .values
            .len(),
    }));
    let syntax_tree = ast::parse(arena, state, &cloned_env, &global_af_info, read)
        .map_err(|e| format!("Syntax error: {}", e))?;
    arena
        .get_activation_frame(state.global_frame)
        .borrow_mut()
        .ensure_index(arena, global_af_info.borrow().entries);
    // println!(" => {:?}", syntax_tree);
    compile_run(arena, state, &syntax_tree)
}

pub fn compile_run(
    arena: &Arena,
    state: &mut VmState,
    syntax_tree: &SyntaxElement,
) -> Result<usize, String> {
    let start_pc = state.code.code_size();
    compile::compile(
        &syntax_tree,
        &mut state.code,
        &state.global_environment,
        false,
        true,
    )
    .map_err(|e| format!("Compilation error: {}", e))?;
    state.code.push(Instruction::Finish);
    // println!(" => {:?}", &state.code[start_pc..state.code.len()]);
    vm::run(arena, &mut state.code, start_pc, state.global_frame)
        .map_err(|e| format!("Runtime error: {}", e))
}
