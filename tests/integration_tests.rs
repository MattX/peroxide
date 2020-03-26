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
use peroxide::heap::RootPtr;
use peroxide::initialize;
use peroxide::parse_compile_run;
use peroxide::read::read_many;
use peroxide::value::Value;
use peroxide::VmState;

fn execute(arena: &mut Arena, vm_state: &mut VmState, code: &str) -> Result<Value, String> {
    execute_rooted(arena, vm_state, code).map(|e| arena.get(e.pp()).clone())
}

fn execute_rooted(
    arena: &mut Arena,
    vm_state: &mut VmState,
    code: &str,
) -> Result<RootPtr, String> {
    let mut results: Vec<_> = read_many(arena, code)?
        .into_iter()
        .map(|read| parse_compile_run(arena, vm_state, read))
        .collect::<Result<Vec<_>, _>>()?;
    results.pop().ok_or("no expressions".into())
}

fn execute_to_vec(
    arena: &mut Arena,
    vm_state: &mut VmState,
    code: &str,
) -> Result<Vec<Value>, String> {
    let val = execute_rooted(arena, vm_state, code)?;
    println!("result of exec: {:?}", val);
    let vec = val.pp().pair_to_vec(arena)?;
    Ok(vec.iter().map(|iv| arena.get(*iv).clone()).collect())
}

#[test]
fn it_adds_two() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(4.into()),
        execute(&mut arena, &mut vm_state, "(+ 2 2)").unwrap()
    );
}

#[test]
fn nested_add() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(2.into()),
        execute(&mut arena, &mut vm_state, "(+ (+ 1 1 1) (- 1 2))").unwrap()
    );
}

#[test]
fn immediate_lambda_args() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(1.into()),
        execute(&mut arena, &mut vm_state, "((lambda (x) x) 1)").unwrap()
    );
}

#[test]
fn immediate_lambda_noargs() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(1.into()),
        execute(&mut arena, &mut vm_state, "((lambda () 1))").unwrap()
    );
}

#[test]
fn shadow() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Symbol("inner".into()),
        execute(
            &mut arena,
            &mut vm_state,
            "((lambda (x) ((lambda (x) x) 'inner)) 'outer)"
        )
        .unwrap()
    );
}

#[test]
fn several_args() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        vec![
            Value::Integer(1.into()),
            Value::Integer(2.into()),
            Value::Integer(3.into())
        ],
        execute_to_vec(
            &mut arena,
            &mut vm_state,
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)"
        )
        .unwrap()
    );
}

#[test]
fn dotted() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        vec![
            Value::Integer(1.into()),
            Value::Integer(2.into()),
            Value::Integer(3.into())
        ],
        execute_to_vec(
            &mut arena,
            &mut vm_state,
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)"
        )
        .unwrap()
    );
}

#[test]
fn global_reference() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Boolean(true),
        execute(&mut arena, &mut vm_state, "(define x #t) x").unwrap()
    );
}

#[test]
fn replace_global_reference() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&mut arena, &mut vm_state, "(define x #t) (define x #f) x").unwrap()
    );
}

#[test]
fn set_global_reference() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&mut arena, &mut vm_state, "(define x #t) (set! x #f) x").unwrap()
    );
}

#[test]
fn forward_global_reference() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(5.into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Boolean(true),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(2.into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(3.into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        vec![Value::Integer(26.into()), Value::Integer((-5).into())],
        execute_to_vec(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Boolean(false),
        execute(&mut arena, &mut vm_state, "(define (set!) #f) (set!)").unwrap()
    );
}

#[test]
fn internal_define() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(5.into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Integer(5.into()),
        execute(&mut arena, &mut vm_state, "(apply + (apply - '(2 3)) '(6))").unwrap()
    );
}

#[test]
fn syntactic_closure() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Symbol("outer".into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    assert_eq!(
        Value::Symbol("outer".into()),
        execute(
            &mut arena,
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
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Symbol("greater".into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(cond ((> 3 2) 'greater)
             ((< 3 2) 'less))"
        )
        .unwrap()
    );
}

#[test]
fn cond2() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Symbol("equal".into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(cond ((> 3 3) 'greater)
      ((< 3 3) 'less)
      (else 'equal))"
        )
        .unwrap()
    );
}

#[test]
fn cond3() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Integer(2.into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(cond ((assv 'b '((a 1) (b 2))) => cadr)\
             (else #f))"
        )
        .unwrap()
    );
}

#[test]
fn cond4() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Symbol("not-one".into()),
        execute(
            &mut arena,
            &mut vm_state,
            "((lambda (x) (cond ((= x 1) 'one) (else 'not-one))) 2)"
        )
        .unwrap()
    );
}

#[test]
fn and() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Integer(4.into()),
            Value::Boolean(true)
        ],
        execute_to_vec(
            &mut arena,
            &mut vm_state,
            "(list\
             (and (= 2 2) (> 2 1))\
             (and (= 2 2) (< 2 1))\
             (and 1 2 3 4)\
             (and))"
        )
        .unwrap()
    );
}

#[test]
fn or() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Integer(1.into()),
            Value::Boolean(false)
        ],
        execute_to_vec(
            &mut arena,
            &mut vm_state,
            "(list\
             (or (= 2 2) (< 2 1))\
             (or (= 3 2) (< 2 1))\
             (or 1 2 3 4)\
             (or))"
        )
        .unwrap()
    );
}

#[test]
fn call_cc() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Integer((-4).into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(%call/cc (lambda (exit)\
             (for-each (lambda (x) (if (< x 0) (exit x))) '(1 2 3 -4 5 6))))"
        )
        .unwrap()
    );
}

#[test]
fn do_macro() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Integer(5.into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(do ((i 0 (+ i 1)))
       ((= i 5) i)
     (display i))"
        )
        .unwrap()
    );
}

#[test]
#[ignore]
fn eval() {
    let mut arena = Arena::default();
    let mut vm_state = VmState::new(&mut arena);
    initialize(&mut arena, &mut vm_state, "src/scheme-lib/init.scm").unwrap();
    assert_eq!(
        Value::Integer(20.into()),
        execute(
            &mut arena,
            &mut vm_state,
            "(let ((f (eval '(lambda (f x) (f x x)) (null-environment 5)))) (f + 10))"
        )
        .unwrap()
    );
}
