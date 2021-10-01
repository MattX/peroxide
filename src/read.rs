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

//! Reader system
//!
//! This file contains methods to turn a stream of tokens into Lisp objects.

use std::cell::{Cell, RefCell};
use std::iter::Peekable;
use std::rc::Rc;

use arena::Arena;
use heap::RootPtr;
use lex::{CodeRange, NumValue, PositionedToken, Token};
use num_complex::Complex;
use num_traits::cast::ToPrimitive;
use util::simplify_numeric;
use value::{Locator, Value};
use {lex, File};

#[derive(Debug)]
pub enum NoReadResult {
    Nothing,
    ReadError { msg: String, locator: Locator },
}

#[derive(Debug, Clone)]
pub struct ReadResult {
    pub ptr: RootPtr,
    pub range: CodeRange,
}

pub struct Reader<'ar> {
    arena: &'ar Arena,

    /// If true, insert [`Value::Locator`] objects at each level.
    locate: bool,
    file: Rc<File>,
}

impl<'ar> Reader<'ar> {
    pub fn new(arena: &'ar Arena, locate: bool, file: Rc<File>) -> Self {
        Self {
            arena,
            locate,
            file,
        }
    }

    pub fn read_tokens(&self, tokens: &[PositionedToken]) -> Result<ReadResult, NoReadResult> {
        if tokens.is_empty() {
            return Err(NoReadResult::Nothing);
        }

        let mut it = tokens.iter().peekable();
        let res = self.do_read(&mut it)?;
        if let Some(s) = it.peek() {
            Err(self.error(format!("unexpected token {:?}", s), s.range))
        } else {
            Ok(res)
        }
    }

    pub fn read_many(&self, code: &str) -> Result<Vec<ReadResult>, NoReadResult> {
        let tokens = lex::lex(code).map_err(|e| self.error(e.msg, e.location))?;
        let segments = lex::segment(tokens).map_err(|e| self.error(e.msg, e.location))?;
        if !segments.remainder.is_empty() {
            return Err(self.error(
                "unterminated expression: dangling tokens",
                segments
                    .remainder
                    .first()
                    .unwrap()
                    .range
                    .merge(segments.remainder.last().unwrap().range),
            ));
        }
        segments
            .segments
            .iter()
            .map(|s| self.read_tokens(s))
            .collect::<Result<Vec<_>, _>>()
    }

    fn do_read<'a, 'b, I>(&self, it: &'a mut Peekable<I>) -> Result<ReadResult, NoReadResult>
    where
        I: Iterator<Item = &'b PositionedToken>,
    {
        if let Some(t) = it.next() {
            match &t.token {
                Token::Num(x) => Ok(self.insert_positioned(read_num_token(x), t.range)),
                Token::Boolean(b) => Ok(self.insert_positioned(Value::Boolean(*b), t.range)),
                Token::Character(c) => Ok(self.insert_positioned(Value::Character(*c), t.range)),
                Token::String(s) => {
                    Ok(self.insert_positioned(Value::String(RefCell::new(s.to_string())), t.range))
                }
                Token::Symbol(s) => {
                    Ok(self.insert_positioned(Value::Symbol(s.to_ascii_lowercase()), t.range))
                }
                Token::OpenParen => self.read_list(it, Some(t.range), t.range),
                Token::OpenByteVector => self.read_bytevec(it, t.range),
                Token::OpenVector => self.read_vec(it, t.range),
                Token::Quote => self.read_quote(it, "quote", t.range),
                Token::QuasiQuote => self.read_quote(it, "quasiquote", t.range),
                Token::Unquote => self.read_quote(it, "unquote", t.range),
                Token::UnquoteSplicing => self.read_quote(it, "unquote-splicing", t.range),
                _ => Err(self.error(format!("unexpected token {:?}", t), t.range)),
            }
        } else {
            panic!("do_parse called with no tokens");
        }
    }

    /// Reads a list.
    ///
    /// The `start` option should be passed iff no elements of the list have yet been read, and
    /// in that case it should contain the CodeRange for the opening `(`.
    ///
    /// The `prev` option should always contain the CodeRange for the last token read. (TODO this
    /// is basically the same as `start`?)
    fn read_list<'a, 'b, I>(
        &self,
        it: &'a mut Peekable<I>,
        start: Option<CodeRange>,
        prev: CodeRange,
    ) -> Result<ReadResult, NoReadResult>
    where
        I: Iterator<Item = &'b PositionedToken>,
    {
        if let Some(&t) = it.peek() {
            match &t.token {
                Token::ClosingParen => {
                    it.next();
                    Ok(self.insert_positioned(Value::EmptyList, prev.merge(t.range)))
                }
                _ => {
                    let first = self.do_read(it)?;
                    let second = if let Some(PositionedToken {
                        token: Token::Dot,
                        range,
                    }) = it.peek()
                    {
                        it.next();
                        let ret = self.do_read(it);
                        let next = it.next();
                        match next {
                            Some(PositionedToken {
                                token: Token::ClosingParen,
                                ..
                            }) => ret,
                            Some(t) => Err(self.error(
                                format!("unexpected token {:?} after dot", t.token),
                                t.range,
                            )),
                            None => Err(self.error("missing token after dot", *range)),
                        }
                    } else {
                        self.read_list(it, None, first.range)
                    }?;
                    let start = start.unwrap_or(t.range);
                    Ok(self.insert_positioned(
                        Value::Pair(Cell::new(first.ptr.pp()), Cell::new(second.ptr.pp())),
                        start.merge(second.range),
                    ))
                }
            }
        } else {
            Err(self.error("unexpected end of list", prev))
        }
    }

    fn read_bytevec<'a, 'b, I>(
        &self,
        it: &'a mut Peekable<I>,
        start: CodeRange,
    ) -> Result<ReadResult, NoReadResult>
    where
        I: Iterator<Item = &'b PositionedToken>,
    {
        let mut result: Vec<u8> = Vec::new();
        let mut last_pos = start;

        let end = loop {
            let t = it.peek();
            match t {
                None => return Err(self.error("unterminated byte vector", start.merge(last_pos))),
                Some(&t) => match &t.token {
                    Token::ClosingParen => {
                        it.next();
                        break t.range;
                    }
                    Token::Num(NumValue::Integer(i)) => {
                        it.next();
                        let b = i.to_u8().ok_or_else(|| {
                            self.error(format!("invalid byte value: {}", i), t.range)
                        })?;
                        result.push(b);
                        last_pos = t.range;
                    }
                    v => {
                        return Err(
                            self.error(format!("non-byte in bytevector literal: {:?}", v), t.range)
                        );
                    }
                },
            }
        };

        Ok(self.insert_positioned(Value::ByteVector(RefCell::new(result)), start.merge(end)))
    }

    fn read_vec<'a, 'b, I>(
        &self,
        it: &'a mut Peekable<I>,
        start: CodeRange,
    ) -> Result<ReadResult, NoReadResult>
    where
        I: Iterator<Item = &'b PositionedToken>,
    {
        let mut roots = Vec::new();
        let mut result = Vec::new();
        let mut last_pos = start;

        let end = loop {
            let t = it.peek();
            match t {
                None => return Err(self.error("unterminated vector", start.merge(last_pos))),
                Some(&t) => match &t.token {
                    Token::ClosingParen => {
                        it.next();
                        break t.range;
                    }
                    _ => {
                        let elem = self.do_read(it)?;
                        result.push(elem.ptr.pp());
                        roots.push(elem);
                        last_pos = t.range;
                    }
                },
            }
        };

        Ok(self.insert_positioned(Value::Vector(RefCell::new(result)), start.merge(end)))
    }

    fn read_quote<'a, 'b, I>(
        &self,
        it: &'a mut Peekable<I>,
        prefix: &'static str,
        start: CodeRange,
    ) -> Result<ReadResult, NoReadResult>
    where
        I: Iterator<Item = &'b PositionedToken>,
    {
        let quoted = self.do_read(it)?;
        let quoted_list_ptr = self.arena.insert_rooted(Value::Pair(
            Cell::new(quoted.ptr.pp()),
            Cell::new(self.arena.empty_list),
        ));
        let quote_sym_ptr = self.arena.insert_rooted(Value::Symbol(prefix.to_string()));
        Ok(self.insert_positioned(
            Value::Pair(
                Cell::new(quote_sym_ptr.pp()),
                Cell::new(quoted_list_ptr.pp()),
            ),
            start.merge(quoted.range),
        ))
    }

    fn insert_positioned(&self, v: Value, range: CodeRange) -> ReadResult {
        let inner = self.arena.insert_rooted(v);
        let ptr = if self.locate {
            self.arena
                .insert_rooted(Value::Located(inner.pp(), Box::new(self.locator(range))))
        } else {
            inner
        };
        ReadResult { ptr, range }
    }

    /// Convenience method to create a [`Locator`] with the current file name.
    fn locator(&self, range: CodeRange) -> Locator {
        Locator {
            file: self.file.clone(),
            range,
        }
    }

    fn error(&self, msg: impl Into<String>, range: CodeRange) -> NoReadResult {
        NoReadResult::ReadError {
            msg: msg.into(),
            locator: self.locator(range),
        }
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
            _ => panic!("complex numbers in rectangular NumValue"),
        },
    };
    simplify_numeric(equalized)
}
