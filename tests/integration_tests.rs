// Copyright 2018-2020 Matthieu Felix
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
use peroxide::read::read_many;
use peroxide::value::Value;
use peroxide::Interpreter;
use peroxide::VmState;

fn execute(arena: &Arena, vm_state: &mut VmState, code: &str) -> Result<Value, String> {
    println!("In execute");
    let res = execute_rooted(arena, vm_state, code).map(|e| (*e.pp()).clone());
    println!("Execute done");
    res
}

fn execute_rooted(arena: &Arena, vm_state: &mut VmState, code: &str) -> Result<RootPtr, String> {
    let mut results: Vec<_> = read_many(arena, code)?
        .into_iter()
        .map(|read| vm_state.parse_compile_run(arena, read))
        .collect::<Result<Vec<_>, _>>()?;
    results.pop().ok_or("no expressions".into())
}

fn execute_to_vec(arena: &Arena, vm_state: &mut VmState, code: &str) -> Result<Vec<Value>, String> {
    let val = execute_rooted(arena, vm_state, code)?;
    let vec = val.pp().list_to_vec()?;
    Ok(vec.iter().map(|&iv| (*iv).clone()).collect())
}

fn magic_execute(code: &str, init: bool) -> Result<Value, String> {
    let interpreter = make_interpreter(init);
    let mut vms = interpreter.as_vm_state();
    // execute(&interpreter.arena, &mut vms, code)
    let res = execute(&interpreter.arena, &mut vms, code);
    drop(vms);
    drop(interpreter);
    res
}

fn magic_execute_to_vec(code: &str, init: bool) -> Result<Vec<Value>, String> {
    let interpreter = make_interpreter(init);
    execute_to_vec(&interpreter.arena, &mut interpreter.as_vm_state(), code)
}

fn make_interpreter(init: bool) -> Interpreter {
    let interpreter = Interpreter::new();
    if init {
        interpreter
            .as_vm_state()
            .initialize(&interpreter.arena, "src/scheme-lib/init.scm")
            .unwrap();
    }
    interpreter
}

#[test]
fn it_adds_two() {
    assert_eq!(
        Value::Integer(4.into()),
        magic_execute("(+ 2 2)", false).unwrap()
    );
}

#[test]
fn nested_add() {
    assert_eq!(
        Value::Integer(2.into()),
        magic_execute("(+ (+ 1 1 1) (- 1 2))", false).unwrap()
    );
}

#[test]
fn immediate_lambda_args() {
    assert_eq!(
        Value::Integer(1.into()),
        magic_execute("((lambda (x) x) 1)", false).unwrap()
    );
}

#[test]
fn immediate_lambda_noargs() {
    assert_eq!(
        Value::Integer(1.into()),
        magic_execute("((lambda () 1))", false).unwrap()
    );
}

#[test]
fn shadow() {
    assert_eq!(
        Value::Symbol("inner".into()),
        magic_execute("((lambda (x) ((lambda (x) x) 'inner)) 'outer)", false).unwrap()
    );
}

#[test]
fn several_args() {
    assert_eq!(
        vec![
            Value::Integer(1.into()),
            Value::Integer(2.into()),
            Value::Integer(3.into())
        ],
        magic_execute_to_vec(
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)",
            false
        )
        .unwrap()
    );
}

#[test]
fn dotted() {
    assert_eq!(
        vec![
            Value::Integer(1.into()),
            Value::Integer(2.into()),
            Value::Integer(3.into())
        ],
        magic_execute_to_vec(
            "(define (list . vals) vals)\
             ((lambda (x y z) (list x y z)) 1 2 3)",
            false
        )
        .unwrap()
    );
}

#[test]
fn global_reference() {
    assert_eq!(
        Value::Boolean(true),
        magic_execute("(define x #t) x", false).unwrap()
    );
}

#[test]
fn replace_global_reference() {
    assert_eq!(
        Value::Boolean(false),
        magic_execute("(define x #t) (define x #f) x", false).unwrap()
    );
}

#[test]
fn set_global_reference() {
    assert_eq!(
        Value::Boolean(false),
        magic_execute("(define x #t) (set! x #f) x", false).unwrap()
    );
}

#[test]
fn forward_global_reference() {
    assert_eq!(
        Value::Integer(5.into()),
        magic_execute(
            "(define (print-x) x)\
             (define x 5)\
             (print-x)",
            false
        )
        .unwrap()
    );
}

#[test]
fn mut_rec() {
    assert_eq!(
        Value::Boolean(true),
        magic_execute(
            "(define (odd? x) (if (= x 0) #f (even? (- x 1))))\
             (define (even? x) (if (= x 0) #t (odd? (- x 1))))\
             (odd? 10001)",
            false
        )
        .unwrap()
    );
}

#[test]
fn set_local() {
    assert_eq!(
        Value::Integer(2.into()),
        magic_execute(
            "(define x 2)\
             ((lambda (x)\
             (set! x 3)\
             x) 1)\
             x",
            false
        )
        .unwrap()
    );
}

#[test]
fn set_local2() {
    assert_eq!(
        Value::Integer(3.into()),
        magic_execute(
            "(define x 2)\
             ((lambda (x)\
             (set! x 3)\
             x) 1)",
            false
        )
        .unwrap()
    );
}

#[test]
fn close_env() {
    assert_eq!(
        vec![Value::Integer(26.into()), Value::Integer((-5).into())],
        magic_execute_to_vec(
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
             (list (counter1 0) (counter2 0))",
            false
        )
        .unwrap()
    );
}

#[test]
fn rename_keyword() {
    assert_eq!(
        Value::Boolean(false),
        magic_execute("(define (set!) #f) (set!)", false).unwrap()
    );
}

#[test]
fn internal_define() {
    assert_eq!(
        Value::Integer(5.into()),
        magic_execute(
            "((lambda ()\
             (define x 5)\
             x))",
            false
        )
        .unwrap()
    );
}

#[test]
fn apply() {
    assert_eq!(
        Value::Integer(5.into()),
        magic_execute("(apply + (apply - '(2 3)) '(6))", false).unwrap()
    );
}

#[test]
fn syntactic_closure() {
    assert_eq!(
        Value::Symbol("outer".into()),
        magic_execute(
            "(define x 'outer)\
             (define-syntax tst\
             (lambda (form usage-env def-env)\
             (define outer-x (make-syntactic-closure def-env '() 'x))\
             outer-x))\
             ((lambda (x)\
             (tst)) 'inner)",
            true
        )
        .unwrap()
    );
}

#[test]
fn let_syntax() {
    assert_eq!(
        Value::Symbol("outer".into()),
        magic_execute(
            "(define x 'outer)\
             (let-syntax ((tst\
             (lambda (form usage-env def-env)\
             (define outer-x (make-syntactic-closure def-env '() 'x))\
             outer-x)))\
             ((lambda (x)\
             (tst)) 'inner))",
            true
        )
        .unwrap()
    );
}

#[test]
fn cond1() {
    assert_eq!(
        Value::Symbol("greater".into()),
        magic_execute(
            "(cond ((> 3 2) 'greater)
             ((< 3 2) 'less))",
            true
        )
        .unwrap()
    );
}

#[test]
fn cond2() {
    assert_eq!(
        Value::Symbol("equal".into()),
        magic_execute(
            "(cond ((> 3 3) 'greater)
      ((< 3 3) 'less)
      (else 'equal))",
            true
        )
        .unwrap()
    );
}

#[test]
fn cond3() {
    assert_eq!(
        Value::Integer(2.into()),
        magic_execute(
            "(cond ((assv 'b '((a 1) (b 2))) => cadr)\
             (else #f))",
            true
        )
        .unwrap()
    );
}

#[test]
fn cond4() {
    assert_eq!(
        Value::Symbol("not-one".into()),
        magic_execute(
            "((lambda (x) (cond ((= x 1) 'one) (else 'not-one))) 2)",
            true
        )
        .unwrap()
    );
}

#[test]
fn and() {
    assert_eq!(
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Integer(4.into()),
            Value::Boolean(true)
        ],
        magic_execute_to_vec(
            "(list\
             (and (= 2 2) (> 2 1))\
             (and (= 2 2) (< 2 1))\
             (and 1 2 3 4)\
             (and))",
            true
        )
        .unwrap()
    );
}

#[test]
fn or() {
    assert_eq!(
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Integer(1.into()),
            Value::Boolean(false)
        ],
        magic_execute_to_vec(
            "(list\
             (or (= 2 2) (< 2 1))\
             (or (= 3 2) (< 2 1))\
             (or 1 2 3 4)\
             (or))",
            true
        )
        .unwrap()
    );
}

#[test]
fn call_cc() {
    assert_eq!(
        Value::Integer((-4).into()),
        magic_execute(
            "(%call/cc (lambda (exit)\
             (for-each (lambda (x) (if (< x 0) (exit x))) '(1 2 3 -4 5 6))))",
            true
        )
        .unwrap()
    );
}

#[test]
fn do_macro() {
    assert_eq!(
        Value::Integer(5.into()),
        magic_execute(
            "(do ((i 0 (+ i 1)))
       ((= i 5) i)
     (display i))",
            true
        )
        .unwrap()
    );
}

#[test]
fn eval() {
    assert_eq!(
        Value::Integer(20.into()),
        magic_execute(
            "(let ((f (eval '(lambda (f x) (f x x)) (null-environment 5)))) (f + 10))",
            true
        )
        .unwrap()
    );
}
