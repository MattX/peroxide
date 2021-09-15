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

use std::cell::{Cell, RefCell};
use std::iter::Peekable;

use arena::Arena;
use heap::RootPtr;
use lex;
use lex::{NumValue, PositionedToken, Token};
use num_complex::Complex;
use num_traits::cast::ToPrimitive;
use util::simplify_numeric;
use value::Value;

#[derive(Debug)]
pub enum ParseResult {
    Nothing,
    ParseError(String),
}

pub fn read_tokens(arena: &Arena, tokens: &[PositionedToken]) -> Result<RootPtr, ParseResult> {
    if tokens.is_empty() {
        return Err(ParseResult::Nothing);
    }

    let mut it = tokens.iter().peekable();
    let res = do_read(arena, &mut it)?;
    if let Some(s) = it.peek() {
        Err(ParseResult::ParseError(format!("Unexpected token {:?}", s)))
    } else {
        Ok(res)
    }
}

pub fn read(arena: &Arena, input: &str) -> Result<RootPtr, String> {
    let tokens = lex::lex(input)?;
    read_tokens(arena, &tokens).map_err(|e| format!("{:?}", e))
}

pub fn read_many(arena: &Arena, code: &str) -> Result<Vec<RootPtr>, String> {
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

fn do_read<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<RootPtr, ParseResult>
where
    I: Iterator<Item = &'b PositionedToken>,
{
    if let Some(t) = it.next() {
        match &t.token {
            Token::Num(x) => Ok(arena.insert_rooted(read_num_token(x))),
            Token::Boolean(b) => Ok(arena.insert_rooted(Value::Boolean(*b))),
            Token::Character(c) => Ok(arena.insert_rooted(Value::Character(*c))),
            Token::String(s) => Ok(arena.insert_rooted(Value::String(RefCell::new(s.to_string())))),
            Token::Symbol(s) => Ok(arena.insert_rooted(Value::Symbol(s.to_ascii_lowercase()))),
            Token::OpenParen => read_list(arena, it),
            Token::OpenByteVector => read_bytevec(arena, it),
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

pub fn read_num_token(t: &NumValue) -> Value {
    let equalized = match t {
        NumValue::Real(r) => Value::Real(*r),
        NumValue::Integer(i) => Value::Integer(i.clone()),
        NumValue::Rational(br) => Value::Rational(Box::new(br.clone())),
        NumValue::Polar(magnitude, phase) => {
            // TODO if phase or magnitude are exact zeros we can do better things
            let magnitude = magnitude.coerce_real();
            let phase = phase.coerce_real();
            Value::ComplexReal(Complex::from_polar(magnitude, phase))
        }
        NumValue::Rectangular(real, imag) => match (real.as_ref(), imag.as_ref()) {
            (NumValue::Real(_), _) | (_, NumValue::Real(_)) => {
                Value::ComplexReal(Complex::new(real.coerce_real(), imag.coerce_real()))
            }
            (NumValue::Rational(_), _) | (_, NumValue::Rational(_)) => Value::ComplexRational(
                Box::new(Complex::new(real.coerce_rational(), imag.coerce_rational())),
            ),
            (NumValue::Integer(real), NumValue::Integer(imag)) => {
                Value::ComplexInteger(Box::new(Complex::new(real.clone(), imag.clone())))
            }
            _ => panic!("Complex numbers in rectangular NumValue"),
        },
    };
    simplify_numeric(equalized)
}

fn read_list<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<RootPtr, ParseResult>
where
    I: Iterator<Item = &'b PositionedToken>,
{
    if let Some(&t) = it.peek() {
        match &t.token {
            Token::ClosingParen => {
                it.next();
                Ok(arena.insert_rooted(Value::EmptyList))
            }
            _ => {
                let first = do_read(arena, it)?;
                let second = if let Some(PositionedToken {
                    token: Token::Dot, ..
                }) = it.peek()
                {
                    it.next();
                    let ret = do_read(arena, it);
                    let next = it.next();
                    if let Some(PositionedToken {
                        token: Token::ClosingParen,
                        ..
                    }) = next
                    {
                        ret
                    } else {
                        Err(ParseResult::ParseError(format!(
                            "Unexpected token {:?} after dot.",
                            next
                        )))
                    }
                } else {
                    read_list(arena, it)
                }?;
                Ok(arena.insert_rooted(Value::Pair(Cell::new(first.pp()), Cell::new(second.pp()))))
            }
        }
    } else {
        Err(ParseResult::ParseError(
            "Unexpected end of list.".to_string(),
        ))
    }
}

fn read_bytevec<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<RootPtr, ParseResult>
where
    I: Iterator<Item = &'b PositionedToken>,
{
    let mut result: Vec<u8> = Vec::new();

    if None == it.peek() {
        return Err(ParseResult::ParseError(
            "Unexpected end of vector.".to_string(),
        ));
    }

    while let Some(&t) = it.peek() {
        match &t.token {
            Token::ClosingParen => {
                it.next();
                break;
            }
            Token::Num(NumValue::Integer(i)) => {
                it.next();
                let b = i.to_u8().ok_or_else(|| {
                    ParseResult::ParseError(format!("Invalid byte value: {}.", i))
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

    Ok(arena.insert_rooted(Value::ByteVector(RefCell::new(result))))
}

fn read_vec<'a, 'b, I>(arena: &Arena, it: &'a mut Peekable<I>) -> Result<RootPtr, ParseResult>
where
    I: Iterator<Item = &'b PositionedToken>,
{
    let mut roots = Vec::new();
    let mut result = Vec::new();

    if None == it.peek() {
        return Err(ParseResult::ParseError(
            "Unexpected end of vector.".to_string(),
        ));
    }

    while let Some(&t) = it.peek() {
        match &t.token {
            Token::ClosingParen => {
                it.next();
                break;
            }
            _ => {
                let elem = do_read(arena, it)?;
                result.push(elem.pp());
                roots.push(elem);
            }
        }
    }

    Ok(arena.insert_rooted(Value::Vector(RefCell::new(result))))
}

fn read_quote<'a, 'b, I>(
    arena: &Arena,
    it: &'a mut Peekable<I>,
    prefix: &'static str,
) -> Result<RootPtr, ParseResult>
where
    I: Iterator<Item = &'b PositionedToken>,
{
    let quoted = do_read(arena, it)?;
    let quoted_list_ptr = arena.insert_rooted(Value::Pair(
        Cell::new(quoted.pp()),
        Cell::new(arena.empty_list),
    ));
    let quote_sym_ptr = arena.insert_rooted(Value::Symbol(prefix.to_string()));
    Ok(arena.insert_rooted(Value::Pair(
        Cell::new(quote_sym_ptr.pp()),
        Cell::new(quoted_list_ptr.pp()),
    )))
}
