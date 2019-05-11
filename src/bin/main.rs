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
extern crate peroxide;
extern crate rustyline;

use std::env;

use peroxide::arena::Arena;
use peroxide::lex::SegmentationResult;
use peroxide::lex::Token;
use peroxide::repl::GetLineError;
use peroxide::repl::{FileRepl, ReadlineRepl, Repl, StdIoRepl};
use peroxide::VmState;

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

    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    loop {
        if !handle_one_expr_wrap(&mut *repl, &arena, &mut vm_state) {
            break;
        }
    }

    repl.save_history();
    Ok(())
}

// Returns true if the REPL loop should continue, false otherwise.
fn handle_one_expr_wrap(repl: &mut Repl, arena: &Arena, vm_state: &mut VmState) -> bool {
    handle_one_expr(repl, arena, vm_state)
        .map_err(|e| println!("Error: {}", e))
        .unwrap_or(true)
}

fn handle_one_expr(repl: &mut Repl, arena: &Arena, vm_state: &mut VmState) -> Result<bool, String> {
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
        let mut tokenize_result = peroxide::lex::lex(&line)?;
        current_expr_string.push(line);
        pending_expr.append(&mut tokenize_result);

        let SegmentationResult {
            mut segments,
            remainder,
            depth: new_depth,
        } = peroxide::lex::segment(pending_expr)?;
        exprs.append(&mut segments);

        if remainder.is_empty() {
            break;
        }

        depth = new_depth;
        pending_expr = remainder;
    }

    repl.add_to_history(&current_expr_string.join("\n"));
    let _ = rep(arena, vm_state, exprs);
    Ok(true)
}

fn rep(arena: &Arena, vm_state: &mut VmState, toks: Vec<Vec<Token>>) -> Result<(), ()> {
    for token_vector in toks {
        let parse_value = peroxide::read::read_tokens(arena, &token_vector)
            .map_err(|e| println!("Parsing error: {:?}", e))?;

        match peroxide::parse_compile_run(arena, vm_state, parse_value) {
            Ok(v) => println!(" => {}", peroxide::value::pretty_print(arena, v)),
            Err(e) => println!("{}", e),
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
        positional.pop().map(ToString::to_string)
    } else {
        return Err("Too many positional arguments.".into());
    };
    Ok(Options {
        enable_readline,
        input_file,
    })
}
