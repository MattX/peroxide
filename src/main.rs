extern crate rustyline;

use std::cell::RefCell;
use std::io;

use rustyline::Editor;
use rustyline::error::ReadlineError;

use arena::Arena;
use continuation::Continuation;
use continuation::ContinuationType;
use environment::Environment;
use value::Value;
use trampoline::Bounce;
use eval::evaluate;

mod lex;
mod value;
mod parse;
mod arena;
mod environment;
mod continuation;
mod eval;
mod trampoline;

fn main() -> io::Result<()> {
  let mut rl = Editor::<()>::new();
  let mut arena = Arena::new();

  let environment = Value::Environment(RefCell::new(Environment::new(None)));
  let environment_r = arena.intern(environment);

  let cont = Value::Continuation(RefCell::new(Continuation {
    next_r: None,
    typ: ContinuationType::TopLevel,
  }));
  let cont_r = arena.intern(cont);

  if rl.load_history("history.txt").is_err() {
    println!("No previous history.");
  }

  loop {
    let readline = rl.readline(">>> ");
    match readline {
      Ok(line) => {
        rl.add_history_entry(line.as_ref());
        rep(&mut arena, line.as_ref(), environment_r, cont_r);
      }
      Err(ReadlineError::Interrupted) => {
        println!("CTRL-C");
        break;
      }
      Err(ReadlineError::Eof) => {
        println!("CTRL-D");
        break;
      }
      Err(err) => {
        println!("Error: {:?}", err);
        break;
      }
    }
  }
  rl.save_history("history.txt").unwrap();
  Ok(())
}

// Read, eval, print
fn rep(arena: &mut Arena, buffer: &str, environment_r: usize, cont_r: usize) -> () {
  match lex::lex(buffer) {
    Ok(token_vector) => {
      match parse::parse(arena, &token_vector) {
        Ok(value) => {
          let value_r = arena.intern(value);
          let result = evaluate(arena, value_r, environment_r, cont_r)
              .map(|x| value::pretty_print(arena, arena.value_ref(x)));
          match result {
            Ok(x) => println!(" => {}", x),
            Err(x) => println!("Error: {}", x),
          }
        },
        Err(s) => println!("Parsing error: {:?}", s),
      }
    }
    Err(s) => println!("Tokenizing error: {}", s),
  }
}
