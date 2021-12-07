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

extern crate clap;
extern crate core;
extern crate peroxide;
extern crate rustyline;

use std::env;
use std::str::FromStr;

use clap::{App, Arg};
use peroxide::heap::GcMode;
use peroxide::lex::{SegmentationResult, Token};
use peroxide::repl::{FileRepl, GetLineError, ReadlineRepl, Repl, StdIoRepl};
use peroxide::Interpreter;

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
        .map_err(|e| format!("could not parse arguments: {}", e))?;

    let silent = options.input_file.is_some();
    let mut repl: Box<dyn Repl> = match options.input_file {
        Some(f) => Box::new(FileRepl::new(&f)?),
        None => {
            if options.enable_readline {
                Box::new(ReadlineRepl::new(Some("history.txt".to_string())))
            } else {
                Box::new(StdIoRepl {})
            }
        }
    };

    let interpreter = Interpreter::new(options.gc_mode);
    let interruptor_clone = interpreter.interruptor();

    ctrlc::set_handler(move || {
        interruptor_clone.interrupt();
    })
    .map_err(|e| format!("error setting Ctrl+C handler: {}", e.to_string()))?;

    if !options.no_std {
        interpreter.initialize("src/scheme-lib/init.scm")?;
    }
    loop {
        if !handle_one_expr_wrap(&mut *repl, &interpreter, silent) {
            break;
        }
    }

    repl.save_history();
    Ok(())
}

// Returns true if the REPL loop should continue, false otherwise.
fn handle_one_expr_wrap(repl: &mut dyn Repl, vm_state: &Interpreter, silent: bool) -> bool {
    handle_one_expr(repl, vm_state, silent)
        .map_err(|e| println!("Error: {}", e))
        .unwrap_or(true)
}

fn handle_one_expr(
    repl: &mut dyn Repl,
    vm_state: &Interpreter,
    silent: bool,
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
    let _ = rep(vm_state, exprs, silent);
    Ok(true)
}

fn rep(vm_state: &Interpreter, toks: Vec<Vec<Token>>, silent: bool) -> Result<(), ()> {
    for token_vector in toks {
        let parse_value = peroxide::read::read_tokens(&vm_state.arena, &token_vector)
            .map_err(|e| println!("parse error: {:?}", e))?;

        match vm_state.parse_compile_run(parse_value) {
            Ok(v) => {
                if !silent {
                    println!(" => {}", v.pp().pretty_print())
                }
            }
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Options {
    pub enable_readline: bool,
    pub no_std: bool,
    pub input_file: Option<String>,
    pub gc_mode: GcMode,
}

fn parse_args(args: &[&str]) -> Result<Options, String> {
    let matches = App::new("Peroxide")
        .version("0.1")
        .author("Matthieu Felix <matthieufelix@gmail.com>")
        .arg(
            Arg::with_name("no-std")
                .long("no-std")
                .help("Do not load the standard library"),
        )
        .arg(
            Arg::with_name("no-readline")
                .long("no-readline")
                .help("Disable readline library"),
        )
        .arg(
            Arg::with_name("gc-mode")
                .long("gc-mode")
                .possible_values(&["off", "normal", "debug", "debug-heavy"])
                .default_value("normal"),
        )
        .arg(Arg::with_name("input-file").help("Sets the input file to use"))
        .get_matches_from(args);

    Ok(Options {
        enable_readline: !matches.is_present("no-readline"),
        no_std: matches.is_present("no-std"),
        input_file: matches.value_of("input-file").map(|v| v.to_string()),
        gc_mode: GcMode::from_str(matches.value_of("gc-mode").unwrap()).unwrap(),
    })
}
