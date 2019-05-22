// Copyright 2018-2019 Matthieu Felix
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

extern crate peroxide;

use peroxide::arena::Arena;
use peroxide::initialize;
use peroxide::parse_compile_run;
use peroxide::read::read_many;
use peroxide::value::Value;
use peroxide::VmState;

fn execute(arena: &Arena, vm_state: &mut VmState, code: &str) -> Result<Value, String> {
    let mut results: Vec<_> = read_many(arena, code)?
        .iter()
        .map(|read| parse_compile_run(arena, vm_state, *read).map(|v| arena.get(v).clone()))
        .collect::<Result<Vec<_>, _>>()?;
    results.pop().ok_or("No expressions".into())
}

fn execute_to_vec(arena: &Arena, vm_state: &mut VmState, code: &str) -> Result<Vec<Value>, String> {
    let val = execute(arena, vm_state, code)?;
    let vec = val.pair_to_vec(arena)?;
    Ok(vec.iter().map(|iv| arena.get(*iv).clone()).collect())
}

#[test]
fn it_adds_two() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(4),
        execute(&arena, &mut vm_state, "(+ 2 2)").unwrap()
    );
}

#[test]
fn nested_add() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(2),
        execute(&arena, &mut vm_state, "(+ (+ 1 1 1) (- 1 2))").unwrap()
    );
}

#[test]
fn immediate_lambda_args() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(1),
        execute(&arena, &mut vm_state, "((lambda (x) x) 1)").unwrap()
    );
}

#[test]
fn immediate_lambda_noargs() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(1),
        execute(&arena, &mut vm_state, "((lambda () 1))").unwrap()
    );
}

#[test]
fn shadow() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Symbol("inner".into()),
        execute(
            &arena,
            &mut vm_state,
            "((lambda (x) ((lambda (x) x) 'inner)) 'outer)"
        )
        .unwrap()
    );
}

#[test]
fn several_args() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        execute_to_vec(
            &arena,
            &mut vm_state,
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)"
        )
        .unwrap()
    );
}

#[test]
fn dotted() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        execute_to_vec(
            &arena,
            &mut vm_state,
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)"
        )
        .unwrap()
    );
}

#[test]
fn global_reference() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Boolean(true),
        execute(&arena, &mut vm_state, "(define x #t) x").unwrap()
    );
}

#[test]
fn replace_global_reference() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&arena, &mut vm_state, "(define x #t) (define x #f) x").unwrap()
    );
}

#[test]
fn set_global_reference() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&arena, &mut vm_state, "(define x #t) (set! x #f) x").unwrap()
    );
}

#[test]
fn forward_global_reference() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(5),
        execute(
            &arena,
            &mut vm_state,
            "(define (print-x) x)\
             (define x 5)\
             (print-x)"
        )
        .unwrap()
    );
}

#[test]
fn mut_rec() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Boolean(true),
        execute(
            &arena,
            &mut vm_state,
            "(define (odd? x) (if (= x 0) #f (even? (- x 1))))\
             (define (even? x) (if (= x 0) #t (odd? (- x 1))))\
             (odd? 10001)"
        )
        .unwrap()
    );
}

#[test]
fn set_local() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(2),
        execute(
            &arena,
            &mut vm_state,
            "(define x 2)\
             ((lambda (x)\
             (set! x 3)\
             x) 1)\
             x"
        )
        .unwrap()
    );
}

#[test]
fn set_local2() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(3),
        execute(
            &arena,
            &mut vm_state,
            "(define x 2)\
             ((lambda (x)\
             (set! x 3)\
             x) 1)"
        )
        .unwrap()
    );
}

#[test]
fn close_env() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        vec![Value::Integer(26), Value::Integer(-5)],
        execute_to_vec(
            &arena,
            &mut vm_state,
            "(define (list . args) args)\
             (define (make-counter init-value)\
               ((lambda (counter-value)\
                  (lambda (increment)\
                     (set! counter-value (+ counter-value increment))\
                     counter-value))\
                init-value))\
             (define counter1 (make-counter 5))\
             (define counter2 (make-counter -5))
             (counter1 3)\
             (counter1 18)\
             (list (counter1 0) (counter2 0))"
        )
        .unwrap()
    );
}

#[test]
fn rename_keyword() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&arena, &mut vm_state, "(define (set!) #f) (set!)").unwrap()
    );
}

#[test]
fn internal_define() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(5),
        execute(
            &arena,
            &mut vm_state,
            "((lambda ()\
             (define x 5)\
             x))"
        )
        .unwrap()
    );
}

#[test]
fn apply() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Integer(5),
        execute(&arena, &mut vm_state, "(apply + (apply - '(2 3)) '(6))").unwrap()
    );
}

#[test]
fn syntactic_closure() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Symbol("outer".into()),
        execute(
            &arena,
            &mut vm_state,
            "(define x 'outer)\
             (define-syntax tst\
             (lambda (form usage-env def-env)\
             (define outer-x (make-syntactic-closure def-env '() 'x))\
             outer-x))\
             ((lambda (x)\
             (tst)) 'inner)"
        )
        .unwrap()
    );
}

#[test]
fn let_syntax() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    assert_eq!(
        Value::Symbol("outer".into()),
        execute(
            &arena,
            &mut vm_state,
            "(define x 'outer)\
             (let-syntax ((tst\
             (lambda (form usage-env def-env)\
             (define outer-x (make-syntactic-closure def-env '() 'x))\
             outer-x)))\
             ((lambda (x)\
             (tst)) 'inner))"
        )
        .unwrap()
    );
}

#[test]
fn cond1() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    initialize(&arena, &mut vm_state, "src/lib/init.scm").unwrap();
    assert_eq!(
        Value::Symbol("greater".into()),
        execute(
            &arena,
            &mut vm_state,
            "(cond ((> 3 2) 'greater)
             ((< 3 2) 'less))"
        )
        .unwrap()
    );
}

#[test]
fn cond2() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    initialize(&arena, &mut vm_state, "src/lib/init.scm").unwrap();
    assert_eq!(
        Value::Symbol("equal".into()),
        execute(
            &arena,
            &mut vm_state,
            "(cond ((> 3 3) 'greater)
      ((< 3 3) 'less)
      (else 'equal))"
        )
        .unwrap()
    );
}

/*
#[test]
fn cond3() {
    let arena = Arena::default();
    let mut vm_state = VmState::new(&arena);
    initialize(&arena, &mut vm_state, "src/lib/init.scm").unwrap();
    assert_eq!(
        Value::Integer(2),
        execute(
            &arena,
            &mut vm_state,
            "(cond ((assv 'b '((a 1) (b 2))) => cadr)\
             (else #f))"
        )
            .unwrap()
    );
}
*/
