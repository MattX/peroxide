use std::cell::RefCell;
use std::iter::Peekable;
use std::rc::Rc;

use lex::Token;
use value::Value;

#[derive(Debug)]
pub enum ParseResult {
  None,
  Incomplete,
  ParseError(String),
}

pub fn parse(tokens: &[Token]) -> Result<Value, ParseResult> {
  if tokens.is_empty() {
    return Err(ParseResult::None);
  }

  let mut it = tokens.iter().peekable();
  do_parse(&mut it)
}

fn do_parse<'a, 'b, I>(it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
  where I: Iterator<Item=&'b Token> {
  if let Some(t) = it.next() {
    match t {
      Token::Real(r) => Ok(Value::Real(*r)),
      Token::Integer(i) => Ok(Value::Integer(*i)),
      Token::Boolean(b) => Ok(Value::Boolean(*b)),
      Token::Character(c) => Ok(Value::Character(*c)),
      Token::String(s) => Ok(Value::String(s.to_string())),
      Token::Symbol(s) => Ok(Value::Symbol(s.to_string())),
      Token::OpenParen => parse_list(it),
      Token::OpenVector => parse_vec(it),
      Token::Quote => parse_quote(it),
      _ => Err(ParseResult::ParseError(format!("Unexpected token {:?}.", t)))
    }
  } else {
    panic!("do_parse called with no tokens.");
  }
}

fn parse_list<'a, 'b, I>(it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
  where I: Iterator<Item=&'b Token> {
  if let Some(&t) = it.peek() {
    match t {
      Token::ClosingParen => {
        it.next();
        Ok(Value::EmptyList)
      }
      _ => {
        let first = do_parse(it)?;
        let second = if it.peek() == Some(&&Token::Dot) {
          it.next();
          let ret = do_parse(it);
          let next = it.next();
          if next != Some(&&Token::ClosingParen) {
            Err(ParseResult::ParseError(format!("Unexpected token {:?} after dot.", next)))
          } else { ret }
        } else {
          parse_list(it)
        }?;
        Ok(Value::Pair(Rc::new(RefCell::new(first)), Rc::new(RefCell::new(second))))
      }
    }
  } else {
    Err(ParseResult::ParseError(format!("Unexpected end of list.")))
  }
}

fn parse_vec<'a, 'b, I>(it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
  where I: Iterator<Item=&'b Token> {
  let mut result: Vec<Value> = Vec::new();

  if None == it.peek() {
    return Err(ParseResult::ParseError(format!("Unexpected end of vector.")));
  }

  while let Some(&t) = it.peek() {
    match t {
      Token::ClosingParen => {
        it.next();
        break;
      }
      _ => {
        let elem = do_parse(it)?;
        result.push(elem);
      }
    }
  }

  Ok(Value::Vector(result))
}

fn parse_quote<'a, 'b, I>(it: &'a mut Peekable<I>) -> Result<Value, ParseResult>
  where I: Iterator<Item=&'b Token> {
  let quoted = do_parse(it)?;
  Ok(Value::Pair(Rc::new(RefCell::new(Value::Symbol("quote".to_string()))),
                 Rc::new(RefCell::new(quoted))))
}
