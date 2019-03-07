use std::cell::RefCell;
use std::iter::Peekable;

use arena::Arena;
use lex::Token;
use value::Value;

#[derive(Debug)]
pub enum ParseResult {
    Nothing,
    ParseError(String),
}

pub fn parse(arena: &mut Arena, tokens: &[Token]) -> Result<Value, ParseResult> {
    if tokens.is_empty() {
        return Err(ParseResult::Nothing);
    }

    let mut it = tokens.iter().peekable();
    let res = do_parse(arena, &mut it);
    if let Some(s) = it.peek() {
        Err(ParseResult::ParseError(format!("Unexpected token {:?}", s)))
    } else {
        res
    }
}

fn do_parse<'a, 'b, I>(arena: &mut Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    if let Some(t) = it.next() {
        match t {
            Token::Real(r) => Ok(Value::Real(*r)),
            Token::Integer(i) => Ok(Value::Integer(*i)),
            Token::Boolean(b) => Ok(Value::Boolean(*b)),
            Token::Character(c) => Ok(Value::Character(*c)),
            Token::String(s) => Ok(Value::String(s.to_string())),
            Token::Symbol(s) => Ok(Value::Symbol(s.to_string())),
            Token::OpenParen => parse_list(arena, it),
            Token::OpenVector => parse_vec(arena, it),
            Token::Quote => parse_quote(arena, it, "quote"),
            Token::QuasiQuote => parse_quote(arena, it, "quasiquote"),
            Token::Unquote => parse_quote(arena, it, "unquote"),
            Token::UnquoteSplicing => parse_quote(arena, it, "unquote-splicing"),
            _ => Err(ParseResult::ParseError(format!(
                "Unexpected token {:?}.",
                t
            ))),
        }
    } else {
        panic!("do_parse called with no tokens.");
    }
}

fn parse_list<'a, 'b, I>(arena: &mut Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
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
                let first = do_parse(arena, it)?;
                let second = if it.peek() == Some(&&Token::Dot) {
                    it.next();
                    let ret = do_parse(arena, it);
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
                    parse_list(arena, it)
                }?;
                let first_ptr = arena.intern(first);
                let second_ptr = arena.intern(second);
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

fn parse_vec<'a, 'b, I>(arena: &mut Arena, it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    let mut result: Vec<RefCell<usize>> = Vec::new();

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
                let elem = do_parse(arena, it)?;
                let elem_ptr = arena.intern(elem);
                result.push(RefCell::new(elem_ptr));
            }
        }
    }

    Ok(Value::Vector(result))
}

fn parse_quote<'a, 'b, I>(
    arena: &mut Arena,
    it: &'a mut Peekable<I>,
    prefix: &'static str,
) -> Result<Value, ParseResult>
where
    I: Iterator<Item = &'b Token>,
{
    let quoted = do_parse(arena, it)?;
    let quoted_ptr = arena.intern(quoted);
    let empty_list_ptr = arena.intern(Value::EmptyList);
    let quoted_list_ptr = arena.intern(Value::Pair(
        RefCell::new(quoted_ptr),
        RefCell::new(empty_list_ptr),
    ));
    let quote_sym_ptr = arena.intern(Value::Symbol(prefix.to_string()));
    Ok(Value::Pair(
        RefCell::new(quote_sym_ptr),
        RefCell::new(quoted_list_ptr),
    ))
}
