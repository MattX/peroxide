use arena::Arena;
use util::with_check_len;
use value::Value;

pub fn eq_p(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
    let args = with_check_len(args, Some(2), Some(2))?;
    Ok(arena.intern(Value::Boolean(args[0] == args[1])))
}

pub fn eqv_p(arena: &mut Arena, args: Vec<usize>) -> Result<usize, String> {
    #![allow(clippy::float_cmp)]
    let args = with_check_len(args, Some(2), Some(2))?;
    let ans = match (arena.value_ref(args[0]), arena.value_ref(args[1])) {
        // This comparison is in the same order as the R5RS one for ease of
        // verification.
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Real(a), Value::Real(b)) => a == b,
        (Value::Character(a), Value::Character(b)) => a == b,
        (Value::EmptyList, Value::EmptyList) => true,
        (Value::Pair(_, _), Value::Pair(_, _)) => args[0] == args[1],
        (Value::Vector(_), Value::Vector(_)) => args[0] == args[1],
        (Value::String(_), Value::String(_)) => args[0] == args[1],
        (Value::Lambda { .. }, Value::Lambda { .. }) => args[0] == args[1],
        _ => false,
    };
    Ok(arena.intern(Value::Boolean(ans)))
}
