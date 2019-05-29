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
use std::fmt;
use util::check_len;
use value::{pretty_print, Value};

// All ports are ReadWrite to get around the fact that Rust can't convert a ReadWrite to a Read,
// which makes a lot of stuff annoying. This is generally fine as Files are also ReadWrite
// regardless of how they are opened.
trait ReadWrite: std::io::Read + std::io::Write {}

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
    stream: RefCell<Option<Box<dyn ReadWrite>>>,
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
        panic!("Trying to clone Ports.");
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
