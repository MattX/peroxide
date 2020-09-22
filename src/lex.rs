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

use std::iter::Peekable;

use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::identities::Zero;
use num_traits::Pow;

use std::str::Chars;
use util;

const EXPT_MARKERS: [char; 10] = ['e', 'E', 's', 'S', 'f', 'F', 'd', 'D', 'l', 'L'];

/// Represents a token from the input.
///
/// For atoms, i.e. basic types, this is essentially the same as a Value: it holds a tag and the
/// actual value. However, there is no representation for other types like pairs or vectors. The
/// parser is in charge of determining if a sequence of tokens validly represents a Scheme
/// expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Num(NumValue),
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
pub struct PositionedToken {
    pub range: CodeRange,
    pub token: Token,
}

// TODO: delete this
impl From<Token> for PositionedToken {
    fn from(t: Token) -> Self {
        Self {
            range: CodeRange {
                start: (0, 0),
                end: (0, 0),
            },
            token: t,
        }
    }
}

/// (line, char)
type CodePosition = (u32, u32);

/// Represents a range in source code.
///
/// Start is inclusive, end is exclusive.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CodeRange {
    pub start: CodePosition,
    pub end: CodePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NumValue {
    Real(f64),
    Integer(BigInt),
    Rational(BigRational),
    Rectangular(Box<NumValue>, Box<NumValue>),
    Polar(Box<NumValue>, Box<NumValue>),
}

impl NumValue {
    pub fn coerce_real(&self) -> f64 {
        match self {
            NumValue::Real(x) => *x,
            NumValue::Integer(x) => util::integer_to_float(x),
            NumValue::Rational(x) => util::rational_to_f64(x),
            _ => panic!("Can't convert complex to real"),
        }
    }

    pub fn coerce_rational(&self) -> BigRational {
        match self {
            NumValue::Integer(x) => BigRational::new(x.clone(), 1.into()),
            NumValue::Rational(x) => x.clone(),
            _ => panic!("Only ints and rationals can be converted to rational"),
        }
    }
}

/// Turns an str slice into a vector of tokens, or fails with an error message.
pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut it = positioned_chars(input).peekable();
    let mut tokens: Vec<Token> = Vec::new();
    loop {
        consume_leading_spaces(&mut it);
        if let Some(&(_pos, c)) = it.peek() {
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
                if let Some(&(_pos, '@')) = it.peek() {
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

struct PositionedChars<'a> {
    line: u32,
    column: u32,
    characters: Chars<'a>,
}

type PositionedChar = (CodePosition, char);

impl<'a> Iterator for PositionedChars<'a> {
    type Item = PositionedChar;

    fn next(&mut self) -> Option<Self::Item> {
        let next_char = self.characters.next()?;
        if next_char == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(((self.line, self.column), next_char))
    }
}

fn positioned_chars(s: &str) -> PositionedChars {
    PositionedChars {
        line: 1,
        column: 1,
        characters: s.chars(),
    }
}

fn consume_to_newline(it: &mut dyn Iterator<Item = PositionedChar>) {
    for (_pos, c) in it {
        if c == '\n' {
            break;
        }
    }
}

fn consume_leading_spaces<I>(it: &mut Peekable<I>)
where
    I: Iterator<Item = PositionedChar>,
{
    while let Some(&(_pos, c)) = it.peek() {
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
    I: Iterator<Item = PositionedChar>,
{
    let mut result: Vec<char> = Vec::new();
    while let Some(&(_pos, c)) = it.peek() {
        if result.len() < min || (c != '(' && c != ')' && !c.is_whitespace()) {
            result.push(c);
            it.next();
        } else {
            break;
        }
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Exactness {
    Exact,
    Inexact,
    Default,
}

impl Exactness {
    fn convert_rational(self, v: BigRational) -> NumValue {
        match self {
            Exactness::Exact | Exactness::Default => NumValue::Rational(v),
            Exactness::Inexact => NumValue::Real(util::rational_to_f64(&v)),
        }
    }

    fn convert_real(self, v: BigRational) -> NumValue {
        match self {
            Exactness::Exact => NumValue::Rational(v),
            Exactness::Inexact | Exactness::Default => NumValue::Real(util::rational_to_f64(&v)),
        }
    }

    fn convert_integer(self, v: BigInt) -> NumValue {
        match self {
            Exactness::Exact | Exactness::Default => NumValue::Integer(v),
            Exactness::Inexact => NumValue::Real(util::integer_to_float(&v)),
        }
    }
}

fn consume_number<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = PositionedChar>,
{
    parse_number(&take_delimited_token(it, 1), 10, Exactness::Default).map(Token::Num)
}

fn consume_sign_or_dot<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = PositionedChar>,
{
    let token = take_delimited_token(it, 1);
    let token_s: String = token.iter().collect();

    if token_s == "." {
        return Ok(Token::Dot);
    }
    if let Some(c) = token.get(1) {
        if c.is_ascii_digit()
            || token_s == "+i"
            || token_s == "-i"
            || token_s.starts_with("+inf.0")
            || token_s.starts_with("-inf.0")
            || token_s.starts_with("+nan.0")
            || token_s.starts_with("-nan.0")
        {
            return parse_number(&token, 10, Exactness::Default).map(Token::Num);
        }
    }
    Ok(Token::Symbol(token_s))
}

fn consume_hash<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = PositionedChar>,
{
    let (_pos, first) = it.next().unwrap();
    debug_assert_eq!(first, '#');
    if let Some((_pos, c)) = it.next() {
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
                (Some((_, '8')), Some((_, '('))) => Ok(Token::OpenByteVector),
                (a, b) => Err(format!("Unknown token form: `#u{:?}{:?}...", a, b)),
            },
            'i' | 'e' | 'b' | 'o' | 'd' | 'x' => {
                let mut num = take_delimited_token(it, 1);
                num.insert(0, c);
                parse_prefixed_number(&num, None).map(Token::Num)
            }
            _ => Err(format!("Unknown token form: `#{}...`.", c)),
        }
    } else {
        Err("Unexpected end of #-token.".to_string())
    }
}

/// Parses a number that may or may not have prefixes.
pub fn parse_full_number(s: &[char], base: u8) -> Result<NumValue, String> {
    if s[0] == '#' {
        parse_prefixed_number(&s[1..], Some(base))
    } else {
        parse_number(s, base, Exactness::Default)
    }
}

/// Parses a number with prefixes.
///
/// According to the Holy Standard, there can be at most two prefixes, one for the base and one to
/// specify exactness. This method assumes the first hash has been consumed.
///
// TODO the code in here is terrible, doesn't check that there's at most one prefix of each kind.
fn parse_prefixed_number(s: &[char], base: Option<u8>) -> Result<NumValue, String> {
    let mut base = base.unwrap_or(10);
    let mut exactness = Exactness::Default;
    let (prefixes, num_start_index) = if let Some('#') = s.get(1) {
        if s.len() < 4 {
            return Err("Invalid numeric prefix".into());
        }
        (vec![s[0], s[2]], 3)
    } else {
        (vec![s[0]], 1)
    };

    for p in prefixes {
        match p {
            'i' => exactness = Exactness::Inexact,
            'e' => exactness = Exactness::Exact,
            'b' => base = 2,
            'o' => base = 8,
            'd' => base = 10,
            'x' => base = 16,
            _ => return Err(format!("Invalid numeric prefix: {}", p)),
        }
    }

    parse_number(&s[num_start_index..], base, exactness)
}

fn parse_number(s: &[char], base: u8, exactness: Exactness) -> Result<NumValue, String> {
    if let Some(pos) = s.iter().position(|x| *x == '@') {
        // Complex in polar notation
        Ok(NumValue::Polar(
            Box::new(parse_simple_number(&s[..pos], base, exactness)?),
            Box::new(parse_simple_number(&s[pos + 1..], base, exactness)?),
        ))
    } else if let Some('i') = s.last() {
        // Rectangular complex.
        // If there is a separator, it must be somewhere after the 2nd char. Otherwise, this is
        // a pure imaginary number.
        let sep_pos = s.iter().rposition(|x| *x == '+' || *x == '-').unwrap_or(0);
        let real_part = if sep_pos > 0 {
            parse_simple_number(&s[..sep_pos], base, exactness)?
        } else {
            NumValue::Integer(0.into())
        };
        let imag_part = if sep_pos == s.len() - 2 {
            // expression ends in +i or -i.
            NumValue::Integer(if s[sep_pos] == '+' { 1 } else { -1 }.into())
        } else {
            parse_simple_number(&s[sep_pos..s.len() - 1], base, exactness)?
        };
        Ok(NumValue::Rectangular(
            Box::new(real_part),
            Box::new(imag_part),
        ))
    } else {
        parse_simple_number(s, base, exactness)
    }
}

/// Parses a simple type: Integer, Ratio, or Real.
fn parse_simple_number(s: &[char], base: u8, exactness: Exactness) -> Result<NumValue, String> {
    if let Some(pos) = s.iter().position(|x| *x == '/') {
        let numerator =
            parse_integer(&s[..pos], base).ok_or_else(|| "Invalid rational".to_string())?;
        let denominator =
            parse_integer(&s[pos + 1..], base).ok_or_else(|| "Invalid rational".to_string())?;
        if denominator.is_zero() {
            Err(format!("Invalid zero denominator: {}", denominator))
        } else {
            Ok(exactness.convert_rational(BigRational::new(numerator, denominator)))
        }
    } else if let Some(x) = parse_special_real(s) {
        if exactness == Exactness::Exact {
            return Err(format!(
                "{} cannot be cast to an exact value",
                s.iter().collect::<String>()
            ));
        }
        Ok(NumValue::Real(x))
    } else if s.iter().any(|x| *x == '.')
        || (base == 10 && s[1..].iter().any(|x| EXPT_MARKERS.contains(x)))
    {
        if base != 10 {
            return Err(format!(
                "Real is specified in base {}, but only base 10 is supported.",
                base
            ));
        }
        parse_float(s)
            .map(|x| exactness.convert_real(x))
            .ok_or_else(|| format!("Invalid float: {}", s.iter().collect::<String>()))
    } else {
        parse_integer(s, base)
            .map(|x| exactness.convert_integer(x))
            .ok_or_else(|| format!("Invalid integer: {}", s.iter().collect::<String>()))
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
    let base = u32::from(base);
    let digits: Option<Vec<u8>> = trimmed_s
        .iter()
        .map(|x| x.to_digit(base).map(|y| y as u8))
        .collect();
    digits.and_then(|d| BigInt::from_radix_be(sign, &d, base))
}

fn parse_float(s: &[char]) -> Option<BigRational> {
    let exponent_position = s.iter().position(|x| EXPT_MARKERS.contains(x));
    let (mantissa_s, mut exponent) = if let Some(pos) = exponent_position {
        (
            &s[..pos],
            (&s[pos + 1..])
                .iter()
                .collect::<String>()
                .parse::<i64>()
                .ok()?,
        )
    } else {
        (s, 0)
    };
    let (int_str, float_expt) = if let Some(pos) = mantissa_s.iter().position(|x| *x == '.') {
        (
            [&mantissa_s[..pos], &mantissa_s[pos + 1..]].concat(),
            mantissa_s.len() - pos - 1,
        )
    } else {
        (mantissa_s.to_vec(), 0)
    };
    exponent -= float_expt as i64;
    let numerator: BigInt = int_str
        .iter()
        .map(|c| if *c == '#' { '0' } else { *c })
        .collect::<String>()
        .parse()
        .ok()?;
    if exponent > 0 {
        Some(BigRational::new(
            numerator * BigInt::from(10).pow(exponent as u64),
            1.into(),
        ))
    } else {
        Some(BigRational::new(
            numerator,
            BigInt::from(10).pow((-exponent) as u64),
        ))
    }
}

fn parse_special_real(s: &[char]) -> Option<f64> {
    match s.iter().collect::<String>().as_ref() {
        "+inf.0" => Some(std::f64::INFINITY),
        "-inf.0" => Some(std::f64::NEG_INFINITY),
        "+nan.0" | "-nan.0" => Some(std::f64::NAN),
        _ => None,
    }
}

fn consume_string<I>(it: &mut Peekable<I>) -> Result<Token, String>
where
    I: Iterator<Item = PositionedChar>,
{
    let (_, first) = it.next().unwrap();
    debug_assert_eq!(first, '"');

    let mut found_end: bool = false;
    let mut escaped: bool = false;
    let mut result: String = String::new();
    for (_pos, c) in it {
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
        Token::Num(NumValue::Integer(i.into()))
    }

    fn real_tok(r: f64) -> Token {
        Token::Num(NumValue::Real(r))
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
        assert_eq!(lex("#xfe").unwrap(), vec![int_tok(254)]);
        assert!(lex("12x3").is_err());
        assert!(lex("123x").is_err());
    }

    #[test]
    fn lex_float() {
        assert_eq!(lex("123.4567").unwrap(), vec![real_tok(123.4567)]);
        assert_eq!(lex(".4567").unwrap(), vec![real_tok(0.4567)]);
        assert_eq!(lex("0.").unwrap(), vec![real_tok(0.0)]);
        assert_eq!(lex("-0.").unwrap(), vec![real_tok(-0.0)]);
        assert_eq!(lex("0.06").unwrap(), vec![real_tok(0.06)]);
        assert_eq!(lex("0.06d0").unwrap(), vec![real_tok(0.06)]);
        assert_eq!(lex("0.06d2").unwrap(), vec![real_tok(6.0)]);
        assert_eq!(lex("0.06d-2").unwrap(), vec![real_tok(0.0006)]);
        assert_eq!(lex("123#.##").unwrap(), vec![real_tok(1230.0)]);
        assert_eq!(
            lex("-inf.0").unwrap(),
            vec![real_tok(std::f64::NEG_INFINITY)]
        );
        assert_eq!(lex("+inf.0").unwrap(), vec![real_tok(std::f64::INFINITY)]);
        assert!(lex("-0a.").is_err());
        assert!(lex("-0.123d").is_err());
        assert!(lex("0.06e").is_err());
    }

    #[test]
    fn lex_polar() {
        assert_eq!(
            lex("1.2@3/4").unwrap(),
            vec![Token::Num(NumValue::Polar(
                Box::new(NumValue::Real(1.2)),
                Box::new(NumValue::Rational(BigRational::new(3.into(), 4.into())))
            ))]
        );
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
