use arena::Arena;
use value::Value;

pub fn add(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
  let numeric_values: Vec<Value> = args.into_iter()
      .map(|v| verify_numeric(arena.value_ref(v).clone()))
      .collect::<Result<Vec<_>, _>>()?;

  pub fn add2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
      (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia + ib),
      (Value::Real(fa), Value::Real(fb)) => Value::Real(fa + fb),
      _ => panic!("cast_same did not return equal numeric types: ({}, {})", a, b)
    }
  }

  Ok(arena.intern(numeric_values.iter().fold(Value::Integer(0), |a, b| add2(&a, &b))))
}

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