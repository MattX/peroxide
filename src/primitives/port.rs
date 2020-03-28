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

use std::cell::{RefCell, RefMut};
use std::fmt;
use std::io::{Error, ErrorKind, Read};

use num_traits::ToPrimitive;

use arena::Arena;
use heap;
use heap::PoolPtr;
use util::check_len;
use value::Value;

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
                return if i != 0 && e.kind() == ErrorKind::UnexpectedEof {
                    Err(std::io::Error::new(
                        ErrorKind::InvalidData,
                        "stream does not contain valid UTF-8",
                    ))
                } else {
                    Err(e)
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

pub struct FileTextInputPort {
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

pub struct StringOutputPort {
    underlying: String,
}

impl std::io::Write for StringOutputPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let as_str = std::str::from_utf8(buf).map_err(|_| Error::from(ErrorKind::InvalidData))?;
        self.underlying.push_str(as_str);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl OutputPort for StringOutputPort {
    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn is_closed(&self) -> bool {
        false
    }
}

pub enum Port {
    BinaryInputFile(RefCell<Box<dyn BinaryInputPort>>),
    TextInputFile(RefCell<Box<FileTextInputPort>>),
    OutputString(RefCell<StringOutputPort>),
    OutputFile(RefCell<Box<dyn OutputPort>>),
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#<port>")
    }
}

impl Clone for Port {
    fn clone(&self) -> Self {
        panic!("trying to clone a port");
    }
}

impl PartialEq for Port {
    fn eq(&self, _other: &Self) -> bool {
        panic!("trying to compare ports");
    }
}

impl heap::Inventory for Port {
    fn inventory(&self, _v: &mut heap::PtrVec) {}
}

fn is_port(arg: PoolPtr) -> bool {
    arg.try_get_port().is_some()
}

fn is_input_port(arg: PoolPtr) -> bool {
    match arg.try_get_port().expect("not a port") {
        Port::BinaryInputFile(_) | Port::TextInputFile(_) => true,
        _ => false,
    }
}

fn is_output_port(arg: PoolPtr) -> bool {
    match arg.try_get_port().expect("not a port") {
        Port::OutputFile(_) => true,
        _ => false,
    }
}

fn is_binary_port(arg: PoolPtr) -> bool {
    match arg.try_get_port().expect("not a port") {
        Port::BinaryInputFile(_) | Port::OutputFile(_) => true,
        _ => false,
    }
}

fn is_textual_port(arg: PoolPtr) -> bool {
    match arg.try_get_port().expect("not a port") {
        Port::TextInputFile(_) | Port::OutputFile(_) => true,
        _ => false,
    }
}

pub fn port_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn input_port_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(args[0]) && is_input_port(args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn output_port_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(args[0]) && is_output_port(args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn textual_port_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(args[0]) && is_textual_port(args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn binary_port_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let res = is_port(args[0]) && is_binary_port(args[0]);
    Ok(arena.insert(Value::Boolean(res)))
}

pub fn close_port(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let port = args[0]
        .try_get_port()
        .ok_or_else(|| format!("not a port: {}", args[0].pretty_print()))?;
    match port {
        Port::BinaryInputFile(s) => s.borrow_mut().close(),
        Port::TextInputFile(s) => s.borrow_mut().close(),
        Port::OutputFile(s) => s.borrow_mut().close(),
        Port::OutputString(s) => s.borrow_mut().close(),
    }
    .map_err(|e| e.to_string())?;
    Ok(arena.unspecific)
}

pub fn port_open_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let port = args[0]
        .try_get_port()
        .ok_or_else(|| format!("not a port: {}", args[0].pretty_print()))?;
    let v = match port {
        Port::BinaryInputFile(s) => s.borrow().is_closed(),
        Port::TextInputFile(s) => s.borrow().is_closed(),
        Port::OutputFile(s) => s.borrow().is_closed(),
        Port::OutputString(s) => s.borrow().is_closed(),
    };
    Ok(arena.insert(Value::Boolean(v)))
}

// TODO: paths don't have to be strings on most OSes. We should let the user specify arbitrary
//       bytes. The issue is that I don't think Rust really provides a way to convert arbitrary
//       bytes to a path?
fn get_path(val: PoolPtr) -> Option<std::path::PathBuf> {
    match &*val {
        Value::String(s) => Some(std::path::PathBuf::from(s.borrow().clone())),
        _ => None,
    }
}

pub fn open_input_file(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let path =
        get_path(args[0]).ok_or_else(|| format!("not a valid path: {}", args[0].pretty_print()))?;
    let raw_port = FileTextInputPort::new(&path).map_err(|e| e.to_string())?;
    let port = Port::TextInputFile(RefCell::new(Box::new(raw_port)));
    Ok(arena.insert(Value::Port(Box::new(port))))
}

pub fn eof_object(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(0), Some(0))?;
    Ok(arena.eof)
}

pub fn eof_object_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    Ok(arena.insert(Value::Boolean(args[0] == arena.eof)))
}

fn get_open_text_input_port<'a>(
    val: PoolPtr,
) -> Result<RefMut<'a, Box<FileTextInputPort>>, String> {
    let port: &'a Port = match val.long_lived() {
        Value::Port(b) => b,
        _ => return Err(format!("not a port: {}", val.pretty_print())),
    };
    if let Port::TextInputFile(op) = port {
        let port = op.borrow_mut();
        if port.is_closed() {
            Err(format!("port is closed: {}", val.pretty_print()))
        } else {
            Ok(port)
        }
    } else {
        Err(format!("not a text input port: {}", val.pretty_print()))
    }
}

pub fn read_char(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(args[0])?;
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

pub fn peek_char(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(args[0])?;
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

pub fn read_line(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(args[0])?;
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

pub fn char_ready_p(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    let mut port = get_open_text_input_port(args[0])?;
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

pub fn read_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(2), Some(2))?;
    let len = args[0]
        .try_get_integer()
        .ok_or_else(|| format!("Not an integer: {}", args[0].pretty_print()))?;
    let len = len
        .to_usize()
        .ok_or_else(|| format!("Not a valid index: {}", len))?;
    let mut port = get_open_text_input_port(args[1])?;
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

pub fn open_output_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(0), Some(0))?;
    Ok(
        arena.insert(Value::Port(Box::new(Port::OutputString(RefCell::new(
            StringOutputPort {
                underlying: String::new(),
            },
        ))))),
    )
}

pub fn get_output_string(arena: &Arena, args: &[PoolPtr]) -> Result<PoolPtr, String> {
    check_len(args, Some(1), Some(1))?;
    match args[0]
        .try_get_port()
        .ok_or_else(|| format!("not a port: {}", args[0].pretty_print()))?
    {
        Port::OutputString(s) => {
            Ok(arena.insert(Value::String(RefCell::new(s.borrow().underlying.clone()))))
        }
        _ => Err(format!("invalid port type: {}", args[0].pretty_print())),
    }
}
