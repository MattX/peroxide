use rustyline::Editor;
use rustyline::error::ReadlineError;

use std::io::{self, Write};

#[derive(Debug)]
pub enum GetLineError {
  Eof,
  Interrupted,
  Err(String)
}

pub trait Repl {
  fn get_line(&mut self, prompt: &str, prefill: &str) -> Result<String, GetLineError>;
  fn add_to_history(&mut self, data: &str) -> ();
  fn save_history(&mut self) -> ();
}

pub struct ReadlineRepl {
  editor: Editor<()>,
  history_location: Option<String>
}

impl ReadlineRepl {
  pub fn new(history_location: Option<String>) -> ReadlineRepl {
    let mut ed = ReadlineRepl { editor: Editor::<()>::new(), history_location };

    if ed.editor.load_history("history.txt").is_err() {
      println!("No previous history.");
    }

    ed
  }
}

impl Repl for ReadlineRepl {
  fn get_line(&mut self, prompt: &str, prefill: &str) -> Result<String, GetLineError> {
    self.editor.readline_with_initial(prompt, (prefill, ""))
        .map_err(|e| match e {
          ReadlineError::Eof => GetLineError::Eof,
          ReadlineError::Interrupted => GetLineError::Interrupted,
          _ => GetLineError::Err(e.to_string())
        })
  }

  fn add_to_history(&mut self, data: &str) -> () {
    self.editor.add_history_entry(data);
  }

  fn save_history(&mut self) -> () {
    self.history_location.as_ref().map(|hl| self.editor.save_history(hl));
  }
}

pub struct StdIoRepl {}

impl Repl for StdIoRepl {
  fn get_line(&mut self, prompt: &str, _prefill: &str) -> Result<String, GetLineError> {
    print!("{}", prompt);
    io::stdout().flush().map_err(|e| GetLineError::Err(e.to_string()))?;

    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
      Ok(0) => Err(GetLineError::Eof),
      Ok(_) => Ok(buf),
      Err(e) => Err(GetLineError::Err(e.to_string())),
    }
  }

  fn add_to_history(&mut self, _data: &str) -> () {}

  fn save_history(&mut self) -> () {}
}
