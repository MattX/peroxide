use std::error::Error;
use std::iter::Peekable;

/// Represents a token from the stream. This is essentially the same as a value for Atoms, but it
/// doesn't represent nested structures.
#[derive(Debug, PartialEq)]
pub enum Token {
  Real(f64),
  Integer(i64),
  Boolean(bool),
  Character(char),
  Nil,
}

/// Turns an str slice into a vector of tokens.
pub fn lex(input: &str) -> Result<Vec<Token>, String> {
  let mut it = input.chars().peekable();
  let mut tokens: Vec<Token> = Vec::new();
  loop {
    consume_leading_spaces(&mut it);
    if let Some(&c) = it.peek() {
      let token = if c.is_digit(10) || c == '-' || c == '+' || c == '.' {
        consume_number(&mut it)?
      } else if c == '#' {
        consume_hash(&mut it)?
      } else if c == '(' {
        consume_paren(&mut it)?
      } else {
        return Err(format!("Unexpected token start: `{}`.", c));
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

fn cautious_take_while<P, I>(it: &mut Peekable<I>, min: usize, predicate: P) -> Vec<char>
  where P: Fn(&char) -> bool,
        I: Iterator<Item=char> {
  let mut result: Vec<char> = Vec::new();
  while let Some(&c) = it.peek() {
    if result.len() < min || predicate(&c) {
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
  let chars: String = cautious_take_while(it, 1, |c| !c.is_whitespace()).into_iter().collect();
  Ok(
    chars.parse::<i64>()
        .map(|i| Token::Integer(i))
        .or(chars.parse::<f64>()
            .map(|f| Token::Real(f))
            .map_err(|e| e.description().to_string()))?
  )
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
        let seq = cautious_take_while(it, 1, |c| !c.is_whitespace());
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
      _ => Err(format!("Unknown token form: `#{}...`.", c))
    }
  } else {
    Err(format!("Unexpected end of #-token."))
  }
}

fn consume_paren<I>(it: &mut Peekable<I>) -> Result<Token, String>
  where I: Iterator<Item=char> {
  if it.peek() != Some(&'(') {
    panic!("Unexpected first char `{:?}` in consume_hash.", it.next());
  }
  it.next();
  if let Some(&c) = it.peek() {
    if c != ')' {
      Err(format!("Unexpected token `({}`.", c))
    } else {
      it.next();
      Ok(Token::Nil)
    }
  } else {
    Err(format!("Unknown token `(`"))
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
    assert!(lex("+").is_err());
    assert!(lex("-").is_err());
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
  fn lex_nil() {
    assert_eq!(lex("()").unwrap(), vec![Token::Nil]);
  }

  #[test]
  fn lex_errors() {
    assert!(lex("#").is_err());
    assert!(lex("(").is_err());
    assert!(lex("(x").is_err());
  }

  #[test]
  fn lex_several() {
    assert!(lex("    ").unwrap().is_empty());
    assert!(lex("").unwrap().is_empty());
    assert_eq!(lex("  123   #f   ").unwrap(), vec![Token::Integer(123), Token::Boolean(false)]);
  }

  #[test]
  fn lex_spaces() {
    assert_eq!(lex("  123  ").unwrap(), vec![Token::Integer(123)]);
  }
}
