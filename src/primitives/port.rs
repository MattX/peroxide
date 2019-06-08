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

use std::cell::{RefCell, RefMut};
use std::convert::TryFrom;
use std::fmt;
use std::io::{ErrorKind, Read};

use arena::Arena;
use gc;
use util::check_len;
use value::{pretty_print, Value};

pub trait TextInputPort {
    fn ready(&mut self) -> std::io::Result<bool>;
    fn peek(&mut self) -> std::io::Result<char>;
    fn read_one(&mut self) -> std::io::Result<char>;
    fn read_string(&mut self, n: usize) -> std::io::Result<String>;
    fn close(&mut self) -> std::io::Result<()>;
    fn is_closed(&self) -> bool;
}

pub trait BinaryInputPort {
    fn ready(&mut self) -> std::io::Result<bool>;
    fn peek(&mut self) -> std::io::Result<u8>;
    fn read_one(&mut self) -> std::io::Result<u8>;
    fn read_buf(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn close(&mut self) -> std::io::Result<()>;
    fn is_closed(&self) -> bool;
}

pub trait OutputPort: std::io::Write {
    fn close(&mut self) -> std::io::Result<()>;
    fn is_closed(&self) -> bool;
}

fn read_u8_helper(reader: &mut impl Read) -> std::io::Result<u8> {
    let mut byte_buf = [0 as u8];
    let num_read = reader.read(&mut byte_buf)?;
    if num_read == 0 {
        Err(std::io::Error::from(ErrorKind::UnexpectedEof))
    } else {
        Ok(byte_buf[0])
    }
}

fn read_char_helper(reader: &mut impl Read) -> std::io::Result<char> {
    let mut buf = [0 as u8; 4];
    for i in 0..4 {
        let maybe_u8 = read_u8_helper(reader);
        match maybe_u8 {
            Err(e) => {
                if i != 0 && e.kind() == ErrorKind::UnexpectedEof {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidData,
                        "stream does not contain valid UTF-8",
                    ));
                } else {
                    return Err(e);
                }
            }
            Ok(b) => buf[i] = b,
        }
        let uchar = std::char::from_u32(u32::from_le_bytes(buf));
        if let Some(c) = uchar {
            return Ok(c);
        }
    }
    Err(std::io::Error::new(
        ErrorKind::InvalidData,
        "stream does not contain valid UTF-8",
    ))
}

struct FileTextInputPort {
    reader: Option<std::io::BufReader<std::fs::File>>,
    peek_buffer: Option<char>,
}

impl FileTextInputPort {
    fn new(name: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(name)?;
        Ok(Self {
            reader: Some(std::io::BufReader::new(file)),
            peek_buffer: None,
        })
    }
}

impl TextInputPort for FileTextInputPort {
    fn ready(&mut self) -> std::io::Result<bool> {
        Ok(true)
    }

    fn peek(&mut self) -> std::io::Result<char> {
        if let Some(c) = self.peek_buffer {
            Ok(c)
        } else {
            let c = read_char_helper(self.reader.as_mut().unwrap())?;
            self.peek_buffer = Some(c);
            Ok(c)
        }
    }

    fn read_one(&mut self) -> std::io::Result<char> {
        if let Some(c) = self.peek_buffer {
            self.peek_buffer = None;
            Ok(c)
        } else {
            read_char_helper(self.reader.as_mut().unwrap())
        }
    }

    fn read_string(&mut self, n: usize) -> std::io::Result<String> {
        let mut result = String::with_capacity(n); // We will need at least n, maybe more.
        let mut n = n;
        if let Some(c) = self.peek_buffer {
            self.peek_buffer = None;
            n -= 1;
            result.push(c);
        }
        for _ in 0..n {
            match read_char_helper(self.reader.as_mut().unwrap()) {
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                Ok(c) => result.push(c),
            }
        }
        if n != 0 && result.is_empty() {
            Err(std::io::Error::from(ErrorKind::UnexpectedEof))
        } else {
            Ok(result)
        }
    }

    fn close(&mut self) -> std::io::Result<()> {
        self.reader = None;
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.reader.is_none()
    }
}

pub enum Port {
    BinaryInput(RefCell<Box<dyn BinaryInputPort>>),
    TextInput(RefCell<Box<dyn TextInputPort>>),
    Output(RefCell<Box<dyn OutputPort>>),
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#<port>")
    }
}

impl Clone for Port {
    fn clone(&self) -> Self {
        panic!("Trying to clone a Port.");
    }
}

impl PartialEq for Port {
    fn eq(&self, _other: &Self) -> bool {
        panic!("Trying to compare Ports.");
    }
}

impl gc::Inventory for Port {
    fn inventory(&self, _v: &mut gc::PushOnlyVec<usize>) {}
}

fn is_port(arena: &Arena, arg: usize) -> bool {
    arena.try_get_port(arg).is_some()
}

fn is_input_port(arena: &Arena, arg: usize) -> bool {
    match arena.try_get_port(arg).expect("Not a port.") {
        Port::BinaryInput(_) | Port::TextInput(_) => true,
        _ => false,
    }
}

fn is_output_port(arena: &Arena, arg: usize) -> bool {
    match arena.try_get_port(arg).expect("Not a port.") {
        Port::Output(_) => true,
        _ => false,
    }
}

fn is_binary_port(arena: &Arena, arg: usize) -> bool {
    match arena.try_get_port(arg).expect("Not a port.") {
        Port::BinaryInput(_) | Port::Output(_) => true,
        _ => false,
    }
}

fn is_textual_port(arena: &Arena, arg: usize) -> bool {
    match arena.try_get_port(arg).expect("Not a port.") {
        Port::TextInput(_) | Port::Output(_) => true,
        _ => false,
    }
}

pub fn port_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(arena, args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn input_port_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(arena, args[0]) && is_input_port(arena, args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn output_port_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(arena, args[0]) && is_output_port(arena, args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn textual_port_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(arena, args[0]) && is_textual_port(arena, args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn binary_port_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(arena, args[0]) && is_binary_port(arena, args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn close_port(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let port = arena
        .try_get_port(args[0])
        .ok_or_else(|| format!("Not a port: {}", pretty_print(arena, args[0])))?;
    match port {
        Port::BinaryInput(s) => s.borrow_mut().close(),
        Port::TextInput(s) => s.borrow_mut().close(),
        Port::Output(s) => s.borrow_mut().close(),
    }
    .map_err(|e| e.to_string())?;
    Ok(arena.unspecific)
}

pub fn port_open_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let port = arena
        .try_get_port(args[0])
        .ok_or_else(|| format!("Not a port: {}", pretty_print(arena, args[0])))?;
    let v = match port {
        Port::BinaryInput(s) => s.borrow().is_closed(),
        Port::TextInput(s) => s.borrow().is_closed(),
        Port::Output(s) => s.borrow().is_closed(),
    };
    Ok(arena.insert(Value::Boolean(v)))
}

// TODO: paths don't have to be strings on most OSes. We should let the user specify arbitrary
//       bytes. The issue is that I don't think Rust really provides a way to convert arbitrary
//       bytes to a path?
fn get_path(arena: &Arena, val: usize) -> Option<std::path::PathBuf> {
    match arena.get(val) {
        Value::String(s) => Some(std::path::PathBuf::from(s.borrow().clone())),
        _ => None,
    }
}

pub fn open_input_file(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let path = get_path(arena, args[0])
        .ok_or_else(|| format!("Not a valid path: {}", pretty_print(arena, args[0])))?;
    let raw_port = FileTextInputPort::new(&path).map_err(|e| e.to_string())?;
    let port = Port::TextInput(RefCell::new(Box::new(raw_port)));
    Ok(arena.insert(Value::Port(Box::new(port))))
}

pub fn eof_object(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(0), Some(0))?;
    Ok(arena.eof)
}

pub fn eof_object_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(args[0] == arena.eof)))
}

fn get_open_text_input_port(
    arena: &Arena,
    val: usize,
) -> Result<RefMut<Box<dyn TextInputPort>>, String> {
    if let Port::TextInput(op) = arena
        .try_get_port(val)
        .ok_or_else(|| format!("Not a port: {}", pretty_print(arena, val)))?
    {
        let mut port = op.borrow_mut();
        if port.is_closed() {
            Err(format!("Port is closed: {}", pretty_print(arena, val)))
        } else {
            Ok(port)
        }
    } else {
        Err(format!(
            "Not a text input port: {}",
            pretty_print(arena, val)
        ))
    }
}

pub fn read_char(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(arena, args[0])?;
    match port.read_one() {
        Ok(c) => Ok(arena.insert(Value::Character(c))),
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(arena.eof)
            } else {
                Err(e.to_string())
            }
        }
    }
}

pub fn peek_char(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(arena, args[0])?;
    match port.peek() {
        Ok(c) => Ok(arena.insert(Value::Character(c))),
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(arena.eof)
            } else {
                Err(e.to_string())
            }
        }
    }
}

pub fn read_line(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(arena, args[0])?;
    let mut result = String::new();
    loop {
        match port.read_one() {
            Ok('\n') => return Ok(arena.insert(Value::String(RefCell::new(result)))),
            Ok('\r') => {
                if let Ok('\n') = port.peek() {
                    port.read_one().unwrap();
                }
                return Ok(arena.insert(Value::String(RefCell::new(result))));
            }
            Ok(c) => result.push(c),
            Err(e) => {
                if e.kind() == ErrorKind::UnexpectedEof {
                    if result.is_empty() {
                        return Ok(arena.eof);
                    } else {
                        return Ok(arena.insert(Value::String(RefCell::new(result))));
                    }
                } else {
                    return Err(e.to_string());
                }
            }
        }
    }
}

pub fn char_ready_p(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(arena, args[0])?;
    match port.ready() {
        Ok(ready) => Ok(arena.insert(Value::Boolean(ready))),
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(arena.t)
            } else {
                Err(e.to_string())
            }
        }
    }
}

pub fn read_string(arena: &Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    let len = arena
        .try_get_integer(args[0])
        .ok_or_else(|| format!("Not an integer: {}", pretty_print(arena, args[0])))?;
    let len = usize::try_from(len).map_err(|e| format!("Not a valid index: {}: {}", len, e))?;
    let mut port = get_open_text_input_port(arena, args[1])?;
    match port.read_string(len) {
        Ok(s) => Ok(arena.insert(Value::String(RefCell::new(s)))),
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(arena.eof)
            } else {
                Err(e.to_string())
            }
        }
    }
}
