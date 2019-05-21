# Peroxide

A scheme interpreter in Rust. Aims for R5RS compliance. Heavily based
on the interpreter described in _Lisp in Small Pieces_.


## Todo

### Concrete

* ~~Support internal defines~~
* ~~Support let-syntax and letrec-syntax~~
* ~~Support syntactic closures~~
* Add ~~apply~~ and eval
* Fix the checked vs unchecked references
* Write syntax-rules
* Implement the rest of the stdlib
* Implement fixlet optimization
* ~~Implement name lookup on error~~
* Assign names to lambdas when possible
* There are more places where syntactic closures should be stripped.
* Add `call/cc`
* Turn the GC on (oh noooo)
* Keep track of which lines map to which tokens, which map to which
expressions, which map to what bytecode. This will let us have
much better error messages.
* Allow fully disabling rustyline [using features](
https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section).


#### R7RS stretch goals

### Vague

#### Large

* `call/cc`
* Be faster and less stupid

#### Medium

* Quasiquotation
* Standard library
* Maybe support exact and inexact numbers, and complexes and rationals
 * Catch overflows or support bigints
 * Idea: Make a `Value::Numeric` that would then contain the numeric
   subtypes. 
* Internal defines
* Make errors not be strings :)
* Tie bytecode to AST and AST to input
* Tie bytecode to environment (sort of done if I do the thing above)
* Allow commands like `,exit` or `,decompile`


## Useful documentation

* https://github.com/scheme-requests-for-implementation
* [Page on call/cc](http://www.madore.org/~david/computers/callcc.html#sec_whatis)
* https://schemers.org/Documents/Standards/R5RS/HTML/
* https://github.com/ashinn/chibi-scheme/blob/master/tests/r5rs-tests.scm
* https://github.com/kenpratt/rusty_scheme/blob/master/src/interpreter/cps_interpreter.rs
* _Lisp in Small Pieces_
* https://github.com/withoutboats/shifgrethor
* http://community.schemewiki.org/?scheme-faq-language
* [Dybvig, R. Kent, Robert Hieb, and Carl Bruggeman. "Syntactic abstraction in Scheme."
_Lisp and symbolic computation_ 5.4 (1993): 295-326.
](https://www.cs.indiana.edu/~dyb/pubs/LaSC-5-4-pp295-326.pdf)
* https://www.gnu.org/software/mit-scheme/documentation/mit-scheme-ref/Syntactic-Closures.html#Syntactic-Closures
