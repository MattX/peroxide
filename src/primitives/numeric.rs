use arena::Arena;
use util::with_check_len;
use value::Value;

/// Generates a numeric primitive that runs a simple fold. The provided folder must be a function
/// (&Value, &Value) -> Value
macro_rules! prim_fold_0 {
    ($name:ident, $folder:ident, $fold_initial:expr) => {
        pub fn $name(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
            let values = numeric_vec(arena, args)?;
            Ok(arena.intern(values.iter().fold($fold_initial, |a, b| $folder(&a, &b))))
        }
    };
}

prim_fold_0!(add, add2, Value::Integer(0));
fn add2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia + ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa + fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_fold_0!(mul, mul2, Value::Integer(1));
fn mul2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia * ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa * fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

/// TODO: noncompliant: (- 1) shoud return -1, not 1.
/// Like [prim_fold_0], but uses the first element of the list as the fold initializer
macro_rules! prim_fold_1 {
    ($name:ident, $folder:ident) => {
        pub fn $name(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
            let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(1), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            let first = values
                .first()
                .expect("with_check_len is broken my bois")
                .clone();
            Ok(arena.intern(values[1..].iter().fold(first, |a, b| $folder(&a, &b))))
        }
    };
}

prim_fold_1!(sub, sub2);
fn sub2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => Value::Integer(ia - ib),
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa - fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_fold_1!(div, div2);
fn div2(a: &Value, b: &Value) -> Value {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => {
            if ia % ib == 0 {
                Value::Integer(ia / ib)
            } else {
                Value::Real(ia as f64 / ib as f64)
            }
        }
        (Value::Real(fa), Value::Real(fb)) => Value::Real(fa / fb),
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}


/// Generates a numeric primitive that verifies monotonicity. Needs a (&Value, &Value) -> bool
/// function to wrap.
macro_rules! prim_monotonic {
    ($name:ident, $pair:ident) => {
        pub fn $name(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
            let values: Vec<Value> = with_check_len(numeric_vec(arena, args)?, Some(2), None)
                .map_err(|e| format!("{}: {}", stringify!($name), e))?;
            let ans = values.windows(2).all(|x| $pair(&x[0], &x[1]));
            Ok(arena.intern(Value::Boolean(ans)))
        }
    }
}

prim_monotonic!(equal, equal2);
fn equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia == ib,
        (Value::Real(fa), Value::Real(fb)) => fa == fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(less_than, less_than2);
fn less_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia > ib,
        (Value::Real(fa), Value::Real(fb)) => fa > fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(greater_than, greater_than2);
fn greater_than2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia < ib,
        (Value::Real(fa), Value::Real(fb)) => fa < fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(less_than_equal, less_than_equal2);
fn less_than_equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia <= ib,
        (Value::Real(fa), Value::Real(fb)) => fa <= fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
}

prim_monotonic!(greater_than_equal, greater_than_equal2);
fn greater_than_equal2(a: &Value, b: &Value) -> bool {
    match cast_same(a, b) {
        (Value::Integer(ia), Value::Integer(ib)) => ia >= ib,
        (Value::Real(fa), Value::Real(fb)) => fa >= fb,
        _ => panic!(
            "cast_same did not return equal numeric types: ({}, {})",
            a, b
        ),
    }
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
        _ => (as_float(a), as_float(b)),
    }
}

/// Turns a numeric value into a float.
fn as_float(n: &Value) -> Value {
    match n {
        &Value::Integer(i) => Value::Real(i as f64),
        &Value::Real(f) => Value::Real(f),
        _ => panic!("Non-numeric type passed to as_float"),
    }
}
