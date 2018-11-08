extern crate rustyline;

use std::io;

use rustyline::Editor;
use rustyline::error::ReadlineError;

mod lex;

fn main() -> io::Result<()> {
  let mut rl = Editor::<()>::new();
  if rl.load_history("history.txt").is_err() {
    println!("No previous history.");
  }
  loop {
    let readline = rl.readline(">>> ");
    match readline {
      Ok(line) => {
        rl.add_history_entry(line.as_ref());
        rep(line.as_ref());
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
fn rep(buffer: &str) -> () {
  println!("{:?}", lex::lex(buffer));
}
