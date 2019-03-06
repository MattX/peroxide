use arena::Arena;
use value::Value;
use util::with_check_len;

pub fn add(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn add2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia + ib),
      (Value::Real(fa), Value::Real(fb)) => Value::Real(fa + fb),
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values = numeric_vec(arena, args)?;
  Ok(arena.intern(values.iter().fold(Value::Integer(0), |a, b| add2(&a, &b))))
}

pub fn mul(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn mul2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia * ib),
      (Value::Real(fa), Value::Real(fb)) => Value::Real(fa * fb),
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values = numeric_vec(arena, args)?;
  Ok(arena.intern(values.iter().fold(Value::Integer(1), |a, b| mul2(&a, &b))))
}

pub fn sub(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn sub2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia - ib),
      (Value::Real(fa), Value::Real(fb)) => Value::Real(fa - fb),
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(1), None)
      .map_err(|e| format!("-: {}", e))?;
  let first = values.first().expect("with_check_len is broken my bois").clone();
  Ok(arena.intern(values[1..].iter().fold(first, |a, b| sub2(&a, &b))))
}

pub fn div(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn div2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => {
        if ia % ib == 0 {
          Value::Integer(ia / ib)
        } else {
          Value::Real(ia as f64 / ib as f64)
        }
      },
      (Value::Real(fa), Value::Real(fb)) => Value::Real(fa / fb),
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(1), None)
      .map_err(|e| format!("/: {}", e))?;
  let first = values.first().expect("with_check_len is broken my bois").clone();
  Ok(arena.intern(values[1..].iter().fold(first, |a, b| div2(&a, &b))))
}


pub fn equal(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => ia == ib,
      (Value::Real(fa), Value::Real(fb)) => fa == fb,
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(2), None)
      .map_err(|e| format!("=: {}", e))?;
  let ans = values.windows(2).all(|x| equal2(&x[0], &x[1]));
  Ok(arena.intern(Value::Boolean(ans)))
}

pub fn less_than(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn less_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => ia > ib,
      (Value::Real(fa), Value::Real(fb)) => fa > fb,
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(2), None)
      .map_err(|e| format!("=: {}", e))?;
  let ans = values.windows(2).all(|x| less_than2(&x[0], &x[1]));
  Ok(arena.intern(Value::Boolean(ans)))
}

pub fn more_than(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  fn more_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => ia < ib,
      (Value::Real(fa), Value::Real(fb)) => fa < fb,
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(2), None)
      .map_err(|e| format!("=: {}", e))?;
  let ans = values.windows(2).all(|x| more_than2(&x[0], &x[1]));
  Ok(arena.intern(Value::Boolean(ans)))
}


/// Takes an argument list (vector of arena pointers), returns a vector of numeric values or
/// an error.
fn numeric_vec(arena: &mut Arena, args: Vec<usize>) -> Result<Vec<Value>, String> {
  args.into_iter()
      .map(|v| verify_numeric(arena.value_ref(v).clone()))
      .collect::<Result<Vec<_>, _>>()
}

/// Checks that a value is numeric
fn verify_numeric(a: Value) -> Result<Value, String> {
  match a {
    Value::Integer(_) => Ok(a),
    Value::Real(_) => Ok(a),
    _ => Err(format!("{} is not numeric.", a)),
  }
}

/// Casts two numeric values to the same type.
fn cast_same(a: &Value, b: &Value) -> (Value, Value) {
  match (a, b) {
    (&Value::Integer(ia), &Value::Integer(ib)) => (Value::Integer(ia), Value::Integer(ib)),
    _ => (as_float(a), as_float(b))
  }
}

/// Turns a numeric value into a float.
fn as_float(n: &Value) -> Value {
  match n {
    &Value::Integer(i) => Value::Real(i as f64),
    &Value::Real(f) => Value::Real(f),
    _ => panic!("Non-numeric type passed to as_float")
  }
}