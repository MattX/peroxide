use std::error::Error;
use std::iter::Peekable;

/// Represents a token from the input.
///
/// For atoms, i.e. basic types, this is essentially the same as a Value: it holds a tag and the
/// actual value. However, there is no representation for other types like pairs or vectors. The
/// parser is in charge of determining if a sequence of tokens validly represents a Scheme
/// expression.
#[derive(Debug, PartialEq)]
pub enum Token {
  Real(f64),
  Integer(i64),
  Boolean(bool),
  Character(char),
  Symbol(String),
  String(String),
  OpenVector,
  OpenParen,
  ClosingParen,
  Dot,
  Ellipsis,
  Quote,
  QuasiQuote,
  Unquote,
  UnquoteSplicing,
}

/// Turns an str slice into a vector of tokens, or fails with an error message.
pub fn lex(input: &str) -> Result<Vec<Token>, String> {
  let mut it = input.chars().peekable();
  let mut tokens: Vec<Token> = Vec::new();
  loop {
    consume_leading_spaces(&mut it);
    if let Some(&c) = it.peek() {
      if c == ';' {
        break; // comment
      }
      let token = if c.is_digit(10) || c == '.' || c == '-' || c == '+' {
        consume_number(&mut it)?
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
        Token::Unquote //TODO: Unquote-splicing
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

fn consume_leading_spaces<I>(it: &mut Peekable<I>) -> ()
  where I: Iterator<Item=char> {
  while let Some(&c) = it.peek() {
    if c.is_whitespace() {
      it.next();
    } else {
      break;
    }
  }
}

fn take_delimited_token<I>(it: &mut Peekable<I>, min: usize) -> Vec<char>
  where I: Iterator<Item=char> {
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
  where I: Iterator<Item=char> {
  let token: String = take_delimited_token(it, 1).into_iter().collect();

  if token == "+" || token == "-" {
    Ok(Token::Symbol(token))
  } else if token == "." {
    Ok(Token::Dot)
  } else if token == "..." {
    Ok(Token::Ellipsis)
  } else {
    token.parse::<i64>()
        .map(|i| Token::Integer(i))
        .or(token.parse::<f64>()
            .map(|f| Token::Real(f))
            .map_err(|e| e.description().to_string()))
  }
}

fn consume_hash<I>(it: &mut Peekable<I>) -> Result<Token, String>
  where I: Iterator<Item=char> {
  if it.peek() != Some(&'#') {
    panic!("Unexpected first char `{:?}` in consume_hash.", it.next());
  }
  it.next();
  if let Some(c) = it.next() {
    match c {
      '\\' => {
        let seq = take_delimited_token(it, 1);
        match seq.len() {
          0 => Err(format!("Unexpected end of token.")),
          1 => Ok(Token::Character(seq[0])),
          _ => {
            let descriptor: String = seq.into_iter().collect();
            match descriptor.to_lowercase().as_ref() {
              "newline" => Ok(Token::Character('\n')),
              "space" => Ok(Token::Character(' ')),
              _ => Err(format!("Unknown character descriptor: `{}`.", descriptor))
            }
          }
        }
      }
      't' => Ok(Token::Boolean(true)),
      'f' => Ok(Token::Boolean(false)),
      '(' => Ok(Token::OpenVector),
      _ => Err(format!("Unknown token form: `#{}...`.", c))
    }
  } else {
    Err(format!("Unexpected end of #-token."))
  }
}

fn consume_string<I>(it: &mut Peekable<I>) -> Result<Token, String>
  where I: Iterator<Item=char> {
  if it.peek() != Some(&'"') {
    panic!("Unexpected first char `{:?}` in consume_string.", it.next());
  }
  it.next();

  let mut found_end: bool = false;
  let mut escaped: bool = false;
  let mut result: String = String::new();
  while let Some(c) = it.next() {
    if escaped {
      let r = match c {
        'n' => '\n',
        '"' => '"',
        '\\' => '\\',
        _ => return Err(format!("Invalid escape `\\{}`", c))
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


#[cfg(test)]
mod tests {
  use super::*;

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
    assert_eq!(lex("123").unwrap(), vec![Token::Integer(123)]);
    assert_eq!(lex("0").unwrap(), vec![Token::Integer(0)]);
    assert_eq!(lex("-123").unwrap(), vec![Token::Integer(-123)]);
    assert_eq!(lex("+123").unwrap(), vec![Token::Integer(123)]);
    assert!(lex("12d3").is_err());
    assert!(lex("123d").is_err());
  }

  #[test]
  fn lex_float() {
    assert_eq!(lex("123.4567").unwrap(), vec![Token::Real(123.4567)]);
    assert_eq!(lex(".4567").unwrap(), vec![Token::Real(0.4567)]);
    assert_eq!(lex("0.").unwrap(), vec![Token::Real(0.0)]);
    assert_eq!(lex("-0.").unwrap(), vec![Token::Real(-0.0)]);
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
    assert_eq!(lex("()").unwrap(), vec![Token::OpenParen, Token::ClosingParen]);
    assert_eq!(lex(" (  ) ").unwrap(), vec![Token::OpenParen, Token::ClosingParen]);
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
    assert_eq!(lex("  123   #f   ").unwrap(), vec![Token::Integer(123), Token::Boolean(false)]);
    assert_eq!(lex("123)456").unwrap(),
               vec![Token::Integer(123), Token::ClosingParen, Token::Integer(456)]);
  }

  #[test]
  fn lex_symbol() {
    assert_eq!(lex("abc").unwrap(), vec![Token::Symbol("abc".to_string())]);
    assert_eq!(lex("<=").unwrap(), vec![Token::Symbol("<=".to_string())]);
    assert_eq!(lex("+").unwrap(), vec![Token::Symbol("+".to_string())]);
    assert_eq!(lex(".").unwrap(), vec![Token::Dot]);
    assert_eq!(lex("...").unwrap(), vec![Token::Ellipsis]);
  }

  #[test]
  fn lex_string() {
    assert_eq!(lex("\"abcdef\"").unwrap(), vec![Token::String("abcdef".to_string())]);
    assert_eq!(lex("\"abc\\\"def\"").unwrap(), vec![Token::String("abc\"def".to_string())]);
    assert_eq!(lex("\"abc\\\\def\"").unwrap(), vec![Token::String("abc\\def".to_string())]);
    assert_eq!(lex("\"abc\\ndef\"").unwrap(), vec![Token::String("abc\ndef".to_string())]);
  }

  #[test]
  fn lex_spaces() {
    assert_eq!(lex("  123  ").unwrap(), vec![Token::Integer(123)]);
  }
}
