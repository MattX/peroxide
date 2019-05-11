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

use arena::Arena;
use ast::largest_toplevel_reference;
use environment::{ActivationFrame, CombinedEnv, Environment};
use std::cell::RefCell;
use std::rc::Rc;
use value::Value;
use vm::Instruction;

pub mod arena;
pub mod ast;
pub mod compile;
pub mod environment;
pub mod gc;
pub mod lex;
pub mod parse;
pub mod primitives;
pub mod repl;
pub mod util;
pub mod value;
pub mod vm;

pub struct VmState {
    pub arena: Arena,
    pub environment: CombinedEnv,
    pub code: Vec<Instruction>,
}

/// Holding struct for stuff the VM needs.
impl Default for VmState {
    fn default() -> Self {
        let arena = Arena::default();
        let mut environment = CombinedEnv {
            env: Rc::new(RefCell::new(Environment::new(None))),
            frame: arena.insert(Value::ActivationFrame(RefCell::new(ActivationFrame {
                parent: None,
                values: vec![],
            }))),
        };
        primitives::register_primitives(&arena, &mut environment);

        VmState {
            arena,
            environment,
            code: vec![],
        }
    }
}

pub fn parse_compile_run(state: &mut VmState, read: usize) -> Result<usize, String> {
    let syntax_tree = ast::to_syntax_element(&state.arena, &state.environment.env, read, true)
        .map_err(|e| format!("Syntax error: {}", e))?;
    println!(" => {:?}", syntax_tree);
    if let Some(n) = largest_toplevel_reference(&syntax_tree) {
        state
            .arena
            .get_activation_frame(state.environment.frame)
            .borrow_mut()
            .ensure_index(&state.arena, n);
    }
    let start_pc = state.code.len();
    compile::compile(
        &syntax_tree,
        &mut state.code,
        &state.environment.env,
        false,
        true,
    )
    .map_err(|e| format!("Compilation error: {}", e))?;
    state.code.push(Instruction::Finish);
    println!(" => {:?}", &state.code[start_pc..state.code.len()]);
    vm::run(
        &mut state.arena,
        &state.code,
        start_pc,
        state.environment.frame,
    )
    .map_err(|e| format!("Runtime error: {}", e))
}
