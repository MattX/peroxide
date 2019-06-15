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

use std::error::Error;
use std::iter::Peekable;

use num_bigint::{BigInt, Sign};
use num_rational::BigRational;

/// Represents a token from the input.
///
/// For atoms, i.e. basic types, this is essentially the same as a Value: it holds a tag and the
/// actual value. However, there is no representation for other types like pairs or vectors. The
/// parser is in charge of determining if a sequence of tokens validly represents a Scheme
/// expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Num(NumToken),
    Boolean(bool),
    Character(char),
    Symbol(String),
    String(String),
    OpenVector,
    OpenByteVector,
    OpenParen,
    ClosingParen,
    Dot,
    Quote,
    QuasiQuote,
    Unquote,
    UnquoteSplicing,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NumToken {
    pub value: NumValue,
    pub exactness: Exactness,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exactness {
    Exact,
    Inexact,
    Default,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NumValue {
    Real(f64),
    Integer(BigInt),
    Rational(BigRational),
    Rectangular(Box<NumValue>, Box<NumValue>),
    Polar(Box<NumValue>, Box<NumValue>),
}

/// Turns an str slice into a vector of tokens, or fails with an error message.
pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut it = input.chars().peekable();
    let mut tokens: Vec<Token> = Vec::new();
    loop {
        consume_leading_spaces(&mut it);
        if let Some(&c) = it.peek() {
            if c == ';' {
                consume_to_newline(&mut it);
                continue;
            }
            let token = if c.is_digit(10) {
                consume_number(&mut it)?
            } else if c == '.' || c == '-' || c == '+' {
                consume_sign_or_dot(&mut it)?
            } else if c == '#' {
                consume_hash(&mut it)?
            } else if c == '"' {
                consume_string(&mut it)?
            } else if c == '\'' {
                it.next();
                Token::Quote
            } else if c == '(' {
                it.next();
                Token::OpenParen
            } else if c == ')' {
                it.next();
                Token::ClosingParen
            } else if c == '`' {
                it.next();
                Token::QuasiQuote
            } else if c == ',' {
                it.next();
                if it.peek() == Some(&'@') {
                    it.next();
                    Token::UnquoteSplicing
                } else {
                    Token::Unquote
                }
            } else {
                Token::Symbol(take_delimited_token(&mut it, 1).into_iter().collect())
            };
            tokens.push(token);
        } else {
            break;
        }
    }

    Ok(tokens)
}

fn consume_to_newline(it: &mut Iterator<Item = char>) {
    for c in it {
        if c == '\n' {
            break;
        }
    }
}

fn consume_leading_spaces<I>(it: &mut Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(&c) = it.peek() {
        if c.is_whitespace() {
            it.next();
        } else {
            break;
        }
    }
}

/// Reads a delimited token from the given iterator.
///
/// The token will have a minimum of `min` characters; after that, it ends whenever whitespace or
/// an opening or closing parenthesis is encountered.
fn take_delimited_token<I>(it: &mut Peekable<I>, min: usize) -> Vec<char>
where
    I: Iterator<Item = char>,
{
    let mut result: Vec<char> = Vec::new();
    while let Some(&c) = it.peek() {
        if result.len() < min || (c != '(' && c != ')' && !c.is_whitespace()) {
            result.push(c);
            it.next();
        } else {
            break;
        }
    }
    result
}

fn consume_number<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = char>,
{
    let value = parse_number(&take_delimited_token(it, 1), 10)?;
    Ok(Token::Num(NumToken {
        value,
        exactness: Exactness::Default,
    }))
}

// TODO special logic for -i, +i, -inf.0, +inf.0, -nan.0, +nan.0
fn consume_sign_or_dot<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = char>,
{
    let token = take_delimited_token(it, 1);
    let token_s: String = token.iter().collect();

    if token_s == "." {
        return Ok(Token::Dot);
    }
    if let Some(c) = token.get(1) {
        if c.is_ascii_digit() {
            let value = parse_number(&token, 10)?;
            return Ok(Token::Num(NumToken {
                value,
                exactness: Exactness::Default,
            }));
        }
    }
    Ok(Token::Symbol(token_s))
}

fn consume_hash<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = char>,
{
    if it.peek() != Some(&'#') {
        panic!("Unexpected first char `{:?}` in consume_hash.", it.next());
    }
    it.next();
    if let Some(c) = it.next() {
        match c {
            '\\' => {
                let seq = take_delimited_token(it, 1);
                match seq.len() {
                    0 => Err("Unexpected end of token.".to_string()),
                    1 => Ok(Token::Character(seq[0])),
                    _ => {
                        let descriptor: String = seq.into_iter().collect();
                        match descriptor.to_lowercase().as_ref() {
                            "newline" => Ok(Token::Character('\n')),
                            "space" => Ok(Token::Character(' ')),
                            _ => Err(format!("Unknown character descriptor: `{}`.", descriptor)),
                        }
                    }
                }
            }
            't' => Ok(Token::Boolean(true)),
            'f' => Ok(Token::Boolean(false)),
            '(' => Ok(Token::OpenVector),
            'u' => match (it.next(), it.next()) {
                (Some('8'), Some('(')) => Ok(Token::OpenByteVector),
                (a, b) => Err(format!("Unknown token form: `#u{:?}{:?}...", a, b)),
            },
            'i' | 'e' | 'b' | 'o' | 'd' | 'x' => {
                let mut num = take_delimited_token(it, 1);
                num.insert(0, c);
                parse_prefixed_number(&num).map(|x| Token::Num(x))
            }
            _ => Err(format!("Unknown token form: `#{}...`.", c)),
        }
    } else {
        Err("Unexpected end of #-token.".to_string())
    }
}

/// Parses a number with prefixes.
///
/// According to the Holy Standard, there can be at most two prefixes, one for the base and one to
/// specify exactness. This method assumes the first hash has been consumed.
///
// TODO the code in here is terrible, doesn't check that there's at most one prefix of each kind.
fn parse_prefixed_number(s: &[char]) -> Result<NumToken, String> {
    let mut base = 10;
    let mut exactness = Exactness::Default;
    let (prefixes, num_start_index) = if let Some(c) = s.get(2) {
        if s.get(1) != Some(&'#') {
            return Err("Invalid numeric prefix".into());
        }
        (vec![s[0], *c], 3)
    } else {
        (vec![s[0]], 1)
    };

    for p in prefixes {
        match p {
            'i' => exactness = Exactness::Exact,
            'e' => exactness = Exactness::Inexact,
            'b' => base = 2,
            'o' => base = 8,
            'd' => base = 10,
            'x' => base = 16,
            _ => return Err(format!("Invalid numeric prefix: {}", p)),
        }
    }

    let value = parse_number(&s[num_start_index..], base)?;
    Ok(NumToken { value, exactness })
}

fn parse_number(s: &[char], base: u8) -> Result<NumValue, String> {
    if let Some(pos) = s.iter().position(|x| *x == '@') {
        // Complex in polar notation
        let (magnitude_s, phase_s) = s.split_at(pos);
        let phase_s = &phase_s[1..];
        Ok(NumValue::Polar(
            Box::new(parse_simple_number(magnitude_s, base)?),
            Box::new(parse_simple_number(phase_s, base)?),
        ))
    } else if let Some('i') = s.last() {
        // Rectangular complex
        panic!("Can't read rectangular complexes yet.");
    } else {
        parse_simple_number(s, base)
    }
}

/// Parses a simple type: Integer, Ratio, or Real.
fn parse_simple_number(s: &[char], base: u8) -> Result<NumValue, String> {
    if let Some(pos) = s.iter().position(|x| *x == '/') {
        let numerator =
            parse_integer(&s[..pos], base).ok_or_else(|| format!("Invalid rational"))?;
        let denominator =
            parse_integer(&s[pos + 1..], base).ok_or_else(|| format!("Invalid rational"))?;
        Ok(NumValue::Rational(BigRational::new(numerator, denominator)))
    } else if s.iter().find(|x| **x == '.').is_some() {
        let s: String = s.iter().collect();
        if base != 10 {
            return Err(format!(
                "Real is specified in base {}, but only base 10 is supported.",
                base
            ));
        }
        s.parse::<f64>()
            .map(|x| NumValue::Real(x))
            .map_err(|e| format!("Invalid real: {}: {}", s, e.description().to_string()))
    } else {
        parse_integer(s, base)
            .map(|x| NumValue::Integer(x))
            .ok_or_else(|| format!("Invalid integer"))
    }
}

fn parse_integer(s: &[char], base: u8) -> Option<BigInt> {
    let (sign, trimmed_s) = match s[0] {
        '-' => (Sign::Minus, &s[1..]),
        '+' => (Sign::Plus, &s[1..]),
        _ => (Sign::Plus, s),
    };
    // I don't really understand why Rust's char::to_digit uses u32 for radices and digits, but
    // only supports values under 32, which easily fit in an u8. A mystery for another day, I
    // guess.
    let base = base as u32;
    let digits: Option<Vec<u8>> = trimmed_s
        .iter()
        .map(|x| x.to_digit(base).map(|y| y as u8))
        .collect();
    digits.and_then(|d| BigInt::from_radix_be(sign, &d, base))
}

fn consume_string<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = char>,
{
    if it.peek() != Some(&'"') {
        panic!("Unexpected first char `{:?}` in consume_string.", it.next());
    }
    it.next();

    let mut found_end: bool = false;
    let mut escaped: bool = false;
    let mut result: String = String::new();
    for c in it {
        if escaped {
            let r = match c {
                'n' => '\n',
                '"' => '"',
                '\\' => '\\',
                _ => return Err(format!("Invalid escape `\\{}`", c)),
            };
            result.push(r);
        } else if c == '"' {
            found_end = true;
            break;
        } else if c != '\\' {
            result.push(c);
        }
        escaped = !escaped && c == '\\';
    }

    if found_end {
        Ok(Token::String(result))
    } else {
        Err(format!("Unterminated string `\"{}`.", result))
    }
}

// This can be extended to support R7RS comments and other stuff
pub enum BracketType {
    List,
    Vector,
    Quote,
}

pub struct SegmentationResult {
    pub segments: Vec<Vec<Token>>,
    pub remainder: Vec<Token>,
    pub depth: u64,
}

/// Splits a vector of token into a vector of vector of tokens, each of which represents a single
/// expression that can be read.
pub fn segment(toks: Vec<Token>) -> Result<SegmentationResult, String> {
    let mut segments = Vec::new();
    let mut current_segment = Vec::new();
    let mut brackets = Vec::new();

    for tok in toks.into_iter() {
        current_segment.push(tok.clone());

        match brackets.last() {
            Some(BracketType::Quote) => brackets.pop(),
            _ => None,
        };

        let bracket_type = match tok {
            Token::OpenParen => Some(BracketType::List),
            Token::OpenVector => Some(BracketType::Vector),
            Token::OpenByteVector => Some(BracketType::Vector),
            Token::Quote | Token::QuasiQuote | Token::Unquote | Token::UnquoteSplicing => {
                Some(BracketType::Quote)
            }
            _ => None,
        };

        if let Some(t) = bracket_type {
            brackets.push(t);
        } else if let Token::ClosingParen = tok {
            match brackets.pop() {
                Some(BracketType::List) | Some(BracketType::Vector) => (),
                _ => return Err("Unbalanced right parenthesis".into()),
            };
            match brackets.last() {
                Some(BracketType::Quote) => brackets.pop(),
                _ => None,
            };
        }

        if brackets.is_empty() {
            segments.push(current_segment);
            current_segment = Vec::new();
        }
    }

    Ok(SegmentationResult {
        segments,
        remainder: current_segment,
        depth: brackets.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int_tok(i: i64) -> Token {
        Token::Num(NumToken {
            value: NumValue::Integer(i.into()),
            exactness: Exactness::Default,
        })
    }

    fn real_tok(r: f64) -> Token {
        Token::Num(NumToken {
            value: NumValue::Real(r),
            exactness: Exactness::Default,
        })
    }

    #[test]
    fn lex_char() {
        assert_eq!(lex("#\\!").unwrap(), vec![Token::Character('!')]);
        assert_eq!(lex("#\\n").unwrap(), vec![Token::Character('n')]);
        assert_eq!(lex("#\\ ").unwrap(), vec![Token::Character(' ')]);
        assert_eq!(lex("#\\NeWline").unwrap(), vec![Token::Character('\n')]);
        assert_eq!(lex("#\\space").unwrap(), vec![Token::Character(' ')]);
        assert!(lex("#\\defS").is_err());
        assert!(lex("#\\").is_err());
    }

    #[test]
    fn lex_int() {
        assert_eq!(lex("123").unwrap(), vec![int_tok(123)]);
        assert_eq!(lex("0").unwrap(), vec![int_tok(0)]);
        assert_eq!(lex("-123").unwrap(), vec![int_tok(-123)]);
        assert_eq!(lex("+123").unwrap(), vec![int_tok(123)]);
        assert!(lex("12d3").is_err());
        assert!(lex("123d").is_err());
    }

    #[test]
    fn lex_float() {
        assert_eq!(lex("123.4567").unwrap(), vec![real_tok(123.4567)]);
        assert_eq!(lex(".4567").unwrap(), vec![real_tok(0.4567)]);
        assert_eq!(lex("0.").unwrap(), vec![real_tok(0.0)]);
        assert_eq!(lex("-0.").unwrap(), vec![real_tok(-0.0)]);
        assert!(lex("-0a.").is_err());
        assert!(lex("-0.123d").is_err());
    }

    #[test]
    fn lex_bool() {
        assert_eq!(lex("#f").unwrap(), vec![Token::Boolean(false)]);
        assert_eq!(lex("#t").unwrap(), vec![Token::Boolean(true)]);
    }

    #[test]
    fn lex_parens() {
        assert_eq!(
            lex("()").unwrap(),
            vec![Token::OpenParen, Token::ClosingParen]
        );
        assert_eq!(
            lex(" (  ) ").unwrap(),
            vec![Token::OpenParen, Token::ClosingParen]
        );
    }

    #[test]
    fn lex_errors() {
        assert!(lex("#").is_err());
        assert!(lex("\"abc").is_err());
    }

    #[test]
    fn lex_several() {
        assert!(lex("    ").unwrap().is_empty());
        assert!(lex("").unwrap().is_empty());
        assert_eq!(
            lex("  123   #f   ").unwrap(),
            vec![int_tok(123), Token::Boolean(false)]
        );
        assert_eq!(
            lex("123)456").unwrap(),
            vec![int_tok(123), Token::ClosingParen, int_tok(456)]
        );
    }

    #[test]
    fn lex_symbol() {
        assert_eq!(lex("abc").unwrap(), vec![Token::Symbol("abc".to_string())]);
        assert_eq!(lex("<=").unwrap(), vec![Token::Symbol("<=".to_string())]);
        assert_eq!(lex("+").unwrap(), vec![Token::Symbol("+".to_string())]);
        assert_eq!(lex(".").unwrap(), vec![Token::Dot]);
        assert_eq!(lex("...").unwrap(), vec![Token::Symbol("...".to_string())]);
    }

    #[test]
    fn lex_string() {
        assert_eq!(
            lex("\"abcdef\"").unwrap(),
            vec![Token::String("abcdef".to_string())]
        );
        assert_eq!(
            lex("\"abc\\\"def\"").unwrap(),
            vec![Token::String("abc\"def".to_string())]
        );
        assert_eq!(
            lex("\"abc\\\\def\"").unwrap(),
            vec![Token::String("abc\\def".to_string())]
        );
        assert_eq!(
            lex("\"abc\\ndef\"").unwrap(),
            vec![Token::String("abc\ndef".to_string())]
        );
    }

    #[test]
    fn lex_spaces() {
        assert_eq!(lex("  123  ").unwrap(), vec![int_tok(123)]);
    }
}
