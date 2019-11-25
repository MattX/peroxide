# Peroxide

A scheme interpreter in Rust. Aims for R5RS/R7RS compliance. Heavily based
on the interpreter described in _Lisp in Small Pieces_.


## Todo

### Concrete

* Fix failing tests
  * Implement quote and syntax-quote correctly. Quote should strip outermost
    syntactic closures. 
* Add ~~apply~~ and eval
* Implement error handling
  * It can be handled mostly in userspace, but that creates
    extra trickiness around throwing errors from primitives.
* Fix the checked vs unchecked references
* Implement the rest of the stdlib, esp. ports and bytevecs
* Implement libraries
* Allow define-syntax in internal defines
* Implement let (application of lambda) optimization
* Inline primitives
* ~~Implement name lookup on error~~ ⇒ needs to be fixed
* Assign names to lambdas when possible
* Turn the GC on (oh noooo)
* Keep track of which lines map to which tokens, which map to which
expressions, which map to what bytecode. This will let us have
much better error messages.
* Allow fully disabling rustyline [using features](
https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section).


### Done

* ~~Support internal defines~~
* ~~Support let-syntax and letrec-syntax~~
* ~~Support syntactic closures~~
* ~~Implement `dynamic-wind`~~
* ~~Figure out the `syntax-quote` vs `quote` thing~~
  * I think this is mostly a chibi-scheme thing.
    * It is not :) `quote` also strips out the topmost syntactic closure if it exists.
* ~~Write syntax-rules → done by lifting it from Chibi~~
* ~~Add `call/cc`.~~


### Vague

#### Medium

* Maybe support exact and inexact numbers, and complexes and rationals
 * Catch overflows or support bigints
 * Idea: Make a `Value::Numeric` that would then contain the numeric
   subtypes. 
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
* A GC in rust: https://github.com/withoutboats/shifgrethor
* http://community.schemewiki.org/?scheme-faq-language
* [Dybvig, R. Kent, Robert Hieb, and Carl Bruggeman. "Syntactic abstraction in Scheme."
_Lisp and symbolic computation_ 5.4 (1993): 295-326.
](https://www.cs.indiana.edu/~dyb/pubs/LaSC-5-4-pp295-326.pdf)
* https://www.gnu.org/software/mit-scheme/documentation/mit-scheme-ref/Syntactic-Closures.html#Syntactic-Closures
