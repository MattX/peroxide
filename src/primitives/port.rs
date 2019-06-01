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

use arena::Arena;
use std::cell::RefCell;
use std::collections::vec_deque::VecDeque;
use std::fmt;
use std::io::{ErrorKind, Read};
use std::rc::Rc;
use util::check_len;
use value::{pretty_print, Value};

// TODO this whole file is a 1/4-finished mess.

// All ports are Read+Write to get around the fact that Rust can't convert a Read+Write to a Read,
// which makes a lot of stuff annoying. This is generally fine as Files are also ReadWrite
// regardless of how they are opened. Other streams can just throw an error when read if they are
// Write or vice versa.
trait Stream: std::io::Read + std::io::Write {
    fn close(&mut self) -> std::io::Result<()>;

    fn u8_ready(&mut self) -> std::io::Result<bool>;
    fn char_ready(&mut self) -> std::io::Result<bool>;

    fn peek_u8(&mut self) -> std::io::Result<u8>;
    fn peek_char(&mut self) -> std::io::Result<char>;
}

fn read_u8(reader: &mut impl Read) -> std::io::Result<u8> {
    let mut byte_buf = [0 as u8];
    let num_read = reader.read(&mut byte_buf)?;
    if num_read == 0 {
        Err(std::io::Error::from(ErrorKind::UnexpectedEof))
    } else {
        Ok(byte_buf[0])
    }
}

fn read_char(reader: &mut impl Read) -> std::io::Result<char> {
    let mut buf = [0 as u8; 4];
    for i in 0..4 {
        let maybe_u8 = read_u8(reader);
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
        let uchar = std::char::from_u32(u32::from_le_bytes(buf.into()));
        if let Some(c) = uchar {
            return Ok(c);
        }
    }
    return Err(std::io::Error::new(
        ErrorKind::InvalidData,
        "stream does not contain valid UTF-8",
    ));
}

#[derive(Debug)]
struct RcFile {
    file: Rc<RefCell<std::fs::File>>,
}

impl RcFile {
    fn new(f: std::fs::File) -> Self {
        Self {
            file: Rc::new(RefCell::new(f)),
        }
    }
}

impl std::io::Read for RcFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.borrow_mut().read(buf)
    }
}

impl std::io::Write for RcFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.borrow_mut().flush()
    }
}

impl Clone for RcFile {
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
        }
    }
}

struct PeroxideFile {
    file: RcFile,
    reader: std::io::BufReader<RcFile>,
    writer: std::io::BufWriter<RcFile>,
    peek: RefCell<VecDeque<u8>>,
}

impl PeroxideFile {
    fn new(file: std::fs::File) -> Self {
        let rc_file = RcFile::new(file);
        Self {
            file: rc_file.clone(),
            reader: std::io::BufReader::new(rc_file.clone()),
            writer: std::io::BufWriter::new(rc_file.clone()),
            peek: RefCell::new(VecDeque::new()),
        }
    }
}

impl std::io::Read for PeroxideFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut idx = 0;
        while idx < buf.len() {
            if let Some(b) = self.peek.borrow_mut().pop_front() {
                buf[idx] = b;
            } else {
                break;
            }
            idx += 1;
        }
        self.reader.read(&mut buf[idx..]).map(|n_read| n_read + idx)
    }
}

impl std::io::Write for PeroxideFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl Stream for PeroxideFile {
    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn u8_ready(&mut self) -> std::io::Result<bool> {
        Ok(true) // TODO fix
    }

    fn char_ready(&mut self) -> std::io::Result<bool> {
        Ok(true) // TODO fix
    }

    fn peek_u8(&mut self) -> std::io::Result<u8> {
        if let Some(c) = self.peek.borrow_mut().pop_front() {
            return Ok(c);
        } else {
            let c = read_u8(&mut self.reader)?;
            self.peek.borrow_mut().push_back(c);
            Ok(c)
        }
    }

    fn peek_char(&mut self) -> std::io::Result<char> {
        unimplemented!()
    }
}

#[derive(Debug)]
enum PortMode {
    Read,
    Write,
    ReadWrite,
}

impl PortMode {
    fn can_read(&self) -> bool {
        match self {
            PortMode::Read | PortMode::ReadWrite => true,
            PortMode::Write => false,
        }
    }

    fn can_write(&self) -> bool {
        match self {
            PortMode::Write | PortMode::ReadWrite => true,
            PortMode::Read => false,
        }
    }
}

#[derive(Debug)]
enum PortType {
    Text,
    Binary,
    TextBinary,
}

impl PortType {
    fn is_text(&self) -> bool {
        match self {
            PortType::Text | PortType::TextBinary => true,
            PortType::Binary => false,
        }
    }

    fn is_binary(&self) -> bool {
        match self {
            PortType::Binary | PortType::TextBinary => true,
            PortType::Text => false,
        }
    }
}

pub struct Port {
    stream: RefCell<Option<Box<dyn Stream>>>,
    mode: PortMode,
    binary: PortType,
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#<{:?} {:?} port>", self.mode, self.binary)
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

fn is_port(arena: &Arena, arg: usize) -> bool {
    arena.try_get_port(arg).is_some()
}

fn is_input_port(arena: &Arena, arg: usize) -> bool {
    arena
        .try_get_port(arg)
        .expect("Not a port.")
        .mode
        .can_read()
}

fn is_output_port(arena: &Arena, arg: usize) -> bool {
    arena
        .try_get_port(arg)
        .expect("Not a port.")
        .mode
        .can_write()
}

fn is_binary_port(arena: &Arena, arg: usize) -> bool {
    arena
        .try_get_port(arg)
        .expect("Not a port.")
        .binary
        .is_binary()
}

fn is_textual_port(arena: &Arena, arg: usize) -> bool {
    arena
        .try_get_port(arg)
        .expect("Not a port.")
        .binary
        .is_text()
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
    port.stream.replace(None);
    Ok(arena.unspecific)
}
