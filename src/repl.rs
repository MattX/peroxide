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

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

use rustyline::error::ReadlineError;
use rustyline::Editor;

#[derive(Debug)]
pub enum GetLineError {
    Eof,
    Interrupted,
    Err(String),
}

pub trait Repl {
    fn get_line(&mut self, prompt: &str, prefill: &str) -> Result<String, GetLineError>;
    fn add_to_history(&mut self, data: &str);
    fn save_history(&mut self);
}

pub struct ReadlineRepl {
    editor: Editor<()>,
    history_location: Option<String>,
}

impl ReadlineRepl {
    pub fn new(history_location: Option<String>) -> ReadlineRepl {
        let mut ed = ReadlineRepl {
            editor: Editor::<()>::new(),
            history_location,
        };

        if ed.editor.load_history("history.txt").is_err() {
            println!("No previous history.");
        }

        ed
    }
}

impl Repl for ReadlineRepl {
    fn get_line(&mut self, prompt: &str, prefill: &str) -> Result<String, GetLineError> {
        self.editor
            .readline_with_initial(prompt, (prefill, ""))
            .map_err(|e| match e {
                ReadlineError::Eof => GetLineError::Eof,
                ReadlineError::Interrupted => GetLineError::Interrupted,
                _ => GetLineError::Err(e.to_string()),
            })
    }

    fn add_to_history(&mut self, data: &str) {
        self.editor.add_history_entry(data);
    }

    fn save_history(&mut self) {
        self.history_location
            .clone()
            .map(|hl| self.editor.save_history(&hl));
    }
}

pub struct StdIoRepl {}

impl Repl for StdIoRepl {
    fn get_line(&mut self, prompt: &str, _prefill: &str) -> Result<String, GetLineError> {
        print!("{}", prompt);
        io::stdout()
            .flush()
            .map_err(|e| GetLineError::Err(e.to_string()))?;

        let mut buf = String::new();
        match io::stdin().read_line(&mut buf) {
            Ok(0) => Err(GetLineError::Eof),
            Ok(_) => Ok(buf),
            Err(e) => Err(GetLineError::Err(e.to_string())),
        }
    }

    fn add_to_history(&mut self, _data: &str) {}

    fn save_history(&mut self) {}
}

pub struct FileRepl {
    reader: BufReader<File>,
}

impl FileRepl {
    pub fn new(file_name: &str) -> Result<FileRepl, String> {
        let f = File::open(file_name).map_err(|e| e.to_string())?;
        Ok(FileRepl {
            reader: BufReader::new(f),
        })
    }
}

impl Repl for FileRepl {
    fn get_line(&mut self, _prompt: &str, _prefill: &str) -> Result<String, GetLineError> {
        let mut line = String::new();
        let len = self
            .reader
            .read_line(&mut line)
            .map_err(|e| GetLineError::Err(e.to_string()))?;
        match len {
            0 => Err(GetLineError::Eof),
            _ => Ok(line),
        }
    }

    fn add_to_history(&mut self, _data: &str) {}

    fn save_history(&mut self) {}
}
