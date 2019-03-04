extern crate rustyline;

use std::cell::RefCell;
use std::env;
use std::io;

use arena::Arena;
use continuation::Continuation;
use environment::Environment;
use lex::SegmentationResult;
use lex::Token;
use trampoline::evaluate_toplevel;
use value::Value;
use repl::{ReadlineRepl, Repl, StdIoRepl};
use repl::GetLineError;

mod lex;
mod value;
mod parse;
mod arena;
mod environment;
mod continuation;
mod eval;
mod trampoline;
mod repl;

fn main() -> io::Result<()> {
  let args: Vec<String> = env::args().collect();
  let mut repl: Box<Repl> = if args.iter().any(|x| x == "--no-readline") {
    Box::new(StdIoRepl {})
  } else {
    Box::new(ReadlineRepl::new(Some("history.txt".to_string())))
  };

  let mut arena = Arena::new();

  let environment = Value::Environment(RefCell::new(Environment::new(None)));
  let environment_r = arena.intern(environment);

  let cont = Value::Continuation(RefCell::new(Continuation::TopLevel));
  let cont_r = arena.intern(cont);

  loop {
    let result = handle_one_expr_wrap(&mut repl, &mut arena, environment_r, cont_r);
    if !result { break; }
  }

  repl.save_history();
  Ok(())
}

// Returns true if the REPL loop should continue, false otherwise.
fn handle_one_expr_wrap(repl: &mut Box<Repl>, arena: &mut Arena, environment: usize, continuation: usize)
                        -> bool {
  handle_one_expr(repl, arena, environment, continuation)
      .map_err(|e| println!("Error: {}", e))
      .unwrap_or(true)
}

fn handle_one_expr(repl: &mut Box<Repl>, arena: &mut Arena, environment: usize, continuation: usize)
                   -> Result<bool, String> {
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
      Err(GetLineError::Err(s)) => { println!("Readline error: {}", s); return Ok(true) }
      Ok(_) => ()
    };

    let line = line_opt.unwrap();
    let mut tokenize_result = lex::lex(&line)?;
    current_expr_string.push(line);
    pending_expr.append(&mut tokenize_result);

    let SegmentationResult { mut segments, remainder, depth: new_depth } = lex::segment(pending_expr)?;
    exprs.append(&mut segments);

    if remainder.is_empty() {
      break;
    }

    depth = new_depth;
    pending_expr = remainder;
  }

  repl.add_to_history(&current_expr_string.join("\n"));
  rep(arena, exprs, environment, continuation);
  Ok(true)
}

fn rep(arena: &mut Arena, toks: Vec<Vec<Token>>, environment_r: usize, cont_r: usize) -> () {
  for token_vector in toks {
    match parse::parse(arena, &token_vector) {
      Ok(value) => {
        let value_r = arena.intern(value);
        let result = evaluate_toplevel(arena, value_r, environment_r, cont_r)
            .map(|x| arena.value_ref(x).pretty_print(arena));
        match result {
          Ok(x) => println!(" => {}", x),
          Err(x) => println!(" !> {}", x),
        }
      }
      Err(s) => println!("Parsing error: {:?}", s),
    }
  }
}
