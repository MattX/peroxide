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

extern crate bitvec;
extern crate core;
extern crate num_bigint;
extern crate num_complex;
extern crate num_integer;
extern crate num_rational;
extern crate num_traits;
extern crate rustyline;

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use arena::Arena;
use ast::SyntaxElement;
use environment::{ActivationFrame, ActivationFrameInfo, Environment, RcEnv};
use heap::{GcMode, RootPtr};
use read::read_many;
use value::Value;

pub mod arena;
pub mod ast;
pub mod compile;
pub mod environment;
pub mod heap;
pub mod lex;
pub mod primitives;
pub mod read;
pub mod repl;
pub mod util;
pub mod value;
pub mod vm;

pub const ERROR_HANDLER_INDEX: usize = 0;
pub const INPUT_PORT_INDEX: usize = 1;
pub const OUTPUT_PORT_INDEX: usize = 2;

#[derive(Clone, Debug)]
pub struct Interruptor(Arc<AtomicBool>);

impl Interruptor {
    pub fn interrupt(&self) {
        self.0.store(true, Relaxed);
    }
}

// TODO make arena non-pub
/// Structure holding the global state of the interpreter between effective runs of the VM.
pub struct Interpreter {
    global_environment: RcEnv,
    global_frame: RootPtr,
    interruptor: Arc<AtomicBool>,
    // Keep arena last! It must not be dropped before the RootPtr above.
    pub arena: Arena,
}

// Okay this is another dirty hack. This serves to convince the Rust compiler not to automatically
// drop the Interpreter (and thus the Arena) too soon, e.g. in integration tests. If this whole
// library was designed properly, PoolPtrs would have a lifetime not exceeding that of the Arena,
// but unfortunately that is not the case and we are damned to suffer.
impl Drop for Interpreter {
    fn drop(&mut self) {}
}

impl Interpreter {
    pub fn new(gc_mode: GcMode) -> Self {
        let arena = Arena::with_gc_mode(gc_mode);
        let global_environment = Rc::new(RefCell::new(Environment::new(None)));
        let global_frame =
            arena.insert_rooted(Value::ActivationFrame(RefCell::new(ActivationFrame {
                parent: None,
                values: vec![arena.f, arena.f, arena.f],
            })));
        let afi = Rc::new(RefCell::new(ActivationFrameInfo {
            parent: None,
            altitude: 0,
            entries: 0,
        }));
        // If you add any magic values here, make sure to also add them to the actual toplevel
        // frame above too.
        assert_eq!(
            global_environment
                .borrow_mut()
                .define("%error-handler", &afi, true),
            ERROR_HANDLER_INDEX
        );
        assert_eq!(
            global_environment
                .borrow_mut()
                .define("%current-input-port", &afi, true),
            INPUT_PORT_INDEX
        );
        assert_eq!(
            global_environment
                .borrow_mut()
                .define("%current-output-port", &afi, true),
            OUTPUT_PORT_INDEX
        );
        primitives::register_primitives(&arena, &global_environment, &afi, &global_frame);

        Self {
            arena,
            global_environment,
            global_frame,
            interruptor: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn interruptor(&self) -> Interruptor {
        Interruptor(self.interruptor.clone())
    }

    pub fn initialize(&self, fname: &str) -> Result<(), String> {
        let contents = fs::read_to_string(fname).map_err(|e| e.to_string())?;
        let values = read_many(&self.arena, &contents)?;
        //println!("Values: {:?}", values);
        for v in values.into_iter() {
            // println!("eval> {}", pretty_print(arena, v.pp()));
            self.parse_compile_run(v)?;
        }
        self.global_environment.borrow_mut().remove_special();
        Ok(())
    }

    /// High-level interface to parse, compile, and run a value that's been read.
    pub fn parse_compile_run(&self, read: RootPtr) -> Result<RootPtr, String> {
        let cloned_env = self.global_environment.clone();
        let global_af_info = Rc::new(RefCell::new(ActivationFrameInfo {
            parent: None,
            altitude: 0,
            entries: self
                .global_frame
                .pp()
                .get_activation_frame()
                .borrow()
                .values
                .len(),
        }));
        let syntax_tree = ast::parse(&self.arena, self, &cloned_env, &global_af_info, read.pp())
            .map_err(|e| format!("syntax error: {}", e))?;
        self.global_frame
            .pp()
            .get_activation_frame()
            .borrow_mut()
            .ensure_index(&self.arena, global_af_info.borrow().entries);
        // println!(" => {:?}", syntax_tree);
        self.compile_run(&syntax_tree)
    }

    pub fn compile_run(&self, syntax_tree: &SyntaxElement) -> Result<RootPtr, String> {
        let code =
            compile::compile_toplevel(&self.arena, syntax_tree, self.global_environment.clone());
        let code = self.arena.root(code);
        vm::run(code, 0, self.global_frame.pp(), self)
            .map_err(|e| format!("runtime error: {}", e.pp().pretty_print()))
    }
}
