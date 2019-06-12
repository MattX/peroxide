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

use std::cell::RefCell;
use std::convert::TryFrom;
use std::iter::Peekable;

use arena::Arena;
use lex;
use lex::{NumToken, Token};
use value::Value;

#[derive(Debug)]
pub enum ParseResult {
    Nothing,
    ParseError(String),
}

pub fn read_tokens(arena: &Arena, tokens: &[Token]) -> Result<usize, ParseResult> {
    if tokens.is_empty() {
        return Err(ParseResult::Nothing);
    }

    let mut it = tokens.iter().peekable();
    let res = do_read(arena, &mut it)?;
    if let Some(s) = it.peek() {
        Err(ParseResult::ParseError(format!("Unexpected token {:?}", s)))
    } else {
        Ok(arena.insert(res))
    }
}

pub fn read(arena: &Arena, input: &str) -> Result<usize, String> {
    let tokens = lex::lex(input)?;
    read_tokens(arena, &tokens).map_err(|e| format!("{:?}", e))
}

pub fn read_many(arena: &Arena, code: &str) -> Result<Vec<usize>, String> {
    let tokens = lex::lex(code)?;
    let segments = lex::segment(tokens)?;
    if !segments.remainder.is_empty() {
        return Err(format!(
            "Unterminated expression: dangling tokens {:?}",
            segments.remainder
        ));
    }
    segments
        .segments
        .iter()
        .map(|s| read_tokens(arena, s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{:?}", e))
}

fn do_read<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    if let Some(t) = it.next() {
        match t {
            Token::Num(NumToken::Real(r)) => Ok(Value::Real(*r)),
            Token::Num(NumToken::Integer(i)) => Ok(Value::Integer(*i)),
            Token::Boolean(b) => Ok(Value::Boolean(*b)),
            Token::Character(c) => Ok(Value::Character(*c)),
            Token::String(s) => Ok(Value::String(RefCell::new(s.to_string()))),
            Token::Symbol(s) => Ok(Value::Symbol(s.to_string())),
            Token::OpenParen => read_list(arena, it),
            Token::OpenByteVector => read_bytevec(it),
            Token::OpenVector => read_vec(arena, it),
            Token::Quote => read_quote(arena, it, "quote"),
            Token::QuasiQuote => read_quote(arena, it, "quasiquote"),
            Token::Unquote => read_quote(arena, it, "unquote"),
            Token::UnquoteSplicing => read_quote(arena, it, "unquote-splicing"),
            _ => Err(ParseResult::ParseError(format!(
                "Unexpected token {:?}.",
                t
            ))),
        }
    } else {
        panic!("do_parse called with no tokens.");
    }
}

fn read_list<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    if let Some(&t) = it.peek() {
        match t {
            Token::ClosingParen => {
                it.next();
                Ok(Value::EmptyList)
            }
            _ => {
                let first = do_read(arena, it)?;
                let second = if it.peek() == Some(&&Token::Dot) {
                    it.next();
                    let ret = do_read(arena, it);
                    let next = it.next();
                    if next != Some(&&Token::ClosingParen) {
                        Err(ParseResult::ParseError(format!(
                            "Unexpected token {:?} after dot.",
                            next
                        )))
                    } else {
                        ret
                    }
                } else {
                    read_list(arena, it)
                }?;
                let first_ptr = arena.insert(first);
                let second_ptr = arena.insert(second);
                Ok(Value::Pair(
                    RefCell::new(first_ptr),
                    RefCell::new(second_ptr),
                ))
            }
        }
    } else {
        Err(ParseResult::ParseError(
            "Unexpected end of list.".to_string(),
        ))
    }
}

fn read_bytevec<'a, 'b, I>(it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    let mut result: Vec<u8> = Vec::new();

    if None == it.peek() {
        return Err(ParseResult::ParseError(
            "Unexpected end of vector.".to_string(),
        ));
    }

    while let Some(&t) = it.peek() {
        match t {
            Token::ClosingParen => {
                it.next();
                break;
            }
            Token::Num(NumToken::Integer(i)) => {
                it.next();
                let b = u8::try_from(*i).map_err(|_e| {
                    ParseResult::ParseError(format!("Invalid byte literal: {}.", i))
                })?;
                result.push(b);
            }
            v => {
                return Err(ParseResult::ParseError(format!(
                    "Non-byte in bytevector literal: {:?}",
                    v
                )));
            }
        }
    }

    Ok(Value::ByteVector(RefCell::new(result)))
}

fn read_vec<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    let mut result: Vec<usize> = Vec::new();

    if None == it.peek() {
        return Err(ParseResult::ParseError(
            "Unexpected end of vector.".to_string(),
        ));
    }

    while let Some(&t) = it.peek() {
        match t {
            Token::ClosingParen => {
                it.next();
                break;
            }
            _ => {
                let elem = do_read(arena, it)?;
                let elem_ptr = arena.insert(elem);
                result.push(elem_ptr);
            }
        }
    }

    Ok(Value::Vector(RefCell::new(result)))
}

fn read_quote<'a, 'b, I>(
    arena: &Arena,
    it: &'a mut Peekable<I>,
    prefix: &'static str,
) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    let quoted = do_read(arena, it)?;
    let quoted_ptr = arena.insert(quoted);
    let empty_list_ptr = arena.insert(Value::EmptyList);
    let quoted_list_ptr = arena.insert(Value::Pair(
        RefCell::new(quoted_ptr),
        RefCell::new(empty_list_ptr),
    ));
    let quote_sym_ptr = arena.insert(Value::Symbol(prefix.to_string()));
    Ok(Value::Pair(
        RefCell::new(quote_sym_ptr),
        RefCell::new(quoted_list_ptr),
    ))
}
