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

use std::env;

use arena::Arena;
use environment::{ActivationFrame, CombinedEnv, Environment};
use lex::SegmentationResult;
use lex::Token;
use primitives::register_primitives;
use repl::GetLineError;
use repl::{FileRepl, ReadlineRepl, Repl, StdIoRepl};
use std::cell::RefCell;
use std::rc::Rc;
use value::Value;
use vm::Instruction;

mod arena;
mod ast;
mod compile;
mod environment;
mod lex;
mod macroexpand;
mod parse;
mod primitives;
mod repl;
mod util;
mod value;
mod vm;

fn main() {
    let args: Vec<String> = env::args().collect();
    match do_main(args) {
        Err(e) => {
            println!("Error: {}", e);
            std::process::exit(1)
        }
        Ok(()) => std::process::exit(0),
    }
}

fn do_main(args: Vec<String>) -> Result<(), String> {
    let options = parse_args(&args.iter().map(|x| &**x).collect::<Vec<_>>())
        .map_err(|e| format!("Could not parse arguments: {}", e))?;

    let mut repl: Box<Repl> = match options.input_file {
        Some(f) => Box::new(FileRepl::new(&f)?),
        None => {
            if options.enable_readline {
                Box::new(ReadlineRepl::new(Some("history.txt".to_string())))
            } else {
                Box::new(StdIoRepl {})
            }
        }
    };

    let mut arena = Arena::new();
    let mut environment = CombinedEnv {
        env: Rc::new(RefCell::new(Environment::new(None))),
        frame: arena.intern(Value::ActivationFrame(RefCell::new(ActivationFrame {
            parent: None,
            values: vec![],
        }))),
    };
    register_primitives(&mut arena, &mut environment);
    let mut code = Vec::new();

    loop {
        if !handle_one_expr_wrap(&mut *repl, &mut arena, &environment, &mut code) {
            break;
        }
    }

    repl.save_history();
    Ok(())
}

// Returns true if the REPL loop should continue, false otherwise.
fn handle_one_expr_wrap(
    repl: &mut Repl,
    arena: &mut Arena,
    environment: &CombinedEnv,
    code: &mut Vec<Instruction>,
) -> bool {
    handle_one_expr(repl, arena, environment, code)
        .map_err(|e| println!("Error: {}", e))
        .unwrap_or(true)
}

fn handle_one_expr(
    repl: &mut Repl,
    arena: &mut Arena,
    environment: &CombinedEnv,
    code: &mut Vec<Instruction>,
) -> Result<bool, String> {
    let mut current_expr_string: Vec<String> = Vec::new();
    let mut exprs: Vec<Vec<Token>> = Vec::new();
    let mut pending_expr: Vec<Token> = Vec::new();
    let mut depth: u64 = 0;

    loop {
        let line_opt = if pending_expr.is_empty() {
            repl.get_line(">>> ", "")
        } else {
            repl.get_line("... ", &" ".to_string().repeat((depth * 2) as usize))
        };

        match line_opt {
            Err(GetLineError::Eof) => return Ok(false),
            Err(GetLineError::Interrupted) => return Ok(false),
            Err(GetLineError::Err(s)) => {
                println!("Readline error: {}", s);
                return Ok(true);
            }
            Ok(_) => (),
        };

        let line = line_opt.unwrap();
        let mut tokenize_result = lex::lex(&line)?;
        current_expr_string.push(line);
        pending_expr.append(&mut tokenize_result);

        let SegmentationResult {
            mut segments,
            remainder,
            depth: new_depth,
        } = lex::segment(pending_expr)?;
        exprs.append(&mut segments);

        if remainder.is_empty() {
            break;
        }

        depth = new_depth;
        pending_expr = remainder;
    }

    repl.add_to_history(&current_expr_string.join("\n"));
    let _ = rep(arena, exprs, environment, code);
    Ok(true)
}

fn rep(
    arena: &mut Arena,
    toks: Vec<Vec<Token>>,
    environment: &CombinedEnv,
    code: &mut Vec<Instruction>,
) -> Result<(), ()> {
    for token_vector in toks {
        let parse_value =
            parse::parse(arena, &token_vector).map_err(|e| println!("Parsing error: {:?}", e))?;
        let value_r = arena.intern(parse_value);
        let syntax_tree =
            ast::to_syntax_element(arena, value_r).map_err(|e| println!("Syntax error: {}", e))?;
        println!(" => {:?}", syntax_tree);
        let start_pc = code.len();
        compile::compile(&syntax_tree, code, environment.env.clone(), true, true)
            .map_err(|e| println!("Compilation error: {}", e))?;
        code.push(Instruction::Finish);
        println!(" => {:?}", &code[start_pc..code.len()]);
        match vm::run(arena, code, start_pc, environment.frame) {
            Ok(v) => println!(" => {}", arena.value_ref(v).pretty_print(arena)),
            Err(e) => println!("Runtime error: {:?}", e),
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Options {
    pub enable_readline: bool,
    pub input_file: Option<String>,
}

// TODO emit sensible error / warning messages
fn parse_args(args: &[&str]) -> Result<Options, String> {
    let (mut positional, flags): (Vec<&str>, Vec<&str>) =
        args.iter().skip(1).partition(|s| !s.starts_with("--"));

    let enable_readline = !flags.iter().any(|&x| x == "--no-readline");
    let input_file = if positional.len() <= 1 {
        positional.pop().map(|x| x.to_string())
    } else {
        return Err("Too many positional arguments.".into());
    };
    Ok(Options {
        enable_readline,
        input_file,
    })
}
