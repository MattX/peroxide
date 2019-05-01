use arena::Arena;
use std::cell::RefCell;
use util::check_len;
use value::Value;

pub fn pair_p(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    let ans = match arena.value_ref(args[0]) {
        Value::Pair(_, _) => true,
        _ => false,
    };
    Ok(arena.intern(Value::Boolean(ans)))
}

pub fn cons(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    Ok(arena.intern(Value::Pair(RefCell::new(args[0]), RefCell::new(args[1]))))
}

pub fn car(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.value_ref(args[0]) {
        Value::Pair(car, _) => Ok(*car.borrow()),
        _ => Err(format!(
            "Called car on a non-pair: {}",
            arena.value_ref(args[0]).pretty_print(arena)
        )),
    }
}

pub fn cdr(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(1), Some(1))?;
    match arena.value_ref(args[0]) {
        Value::Pair(_, cdr) => Ok(*cdr.borrow()),
        _ => Err(format!(
            "Called cdr on a non-pair: {}",
            arena.value_ref(args[0]).pretty_print(arena)
        )),
    }
}

pub fn set_car_b(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.value_ref(args[0]) {
        Value::Pair(car, _) => Ok(car.replace(args[1])),
        _ => Err(format!(
            "Called set-car! on a non-pair: {}",
            arena.value_ref(args[0]).pretty_print(arena)
        )),
    }
}

pub fn set_cdr_b(arena: &mut Arena, args: &[usize]) -> Result<usize, String> {
    check_len(args, Some(2), Some(2))?;
    match arena.value_ref(args[0]) {
        Value::Pair(_, cdr) => Ok(cdr.replace(args[1])),
        _ => Err(format!(
            "Called set-cdr! on a non-pair: {}",
            arena.value_ref(args[0]).pretty_print(arena)
        )),
    }
}
