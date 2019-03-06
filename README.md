# Peroxide

A scheme interpreter in Rust. Aims for R5RS compliance; written in
continuation-passing style.

## Ramblings

After starting this project, I realized that Rust made it super
cumbersome to represent complex (especially loopy) data structures. Of
course, an interpreter has to handle complex data structures, because
the user could do arbitrary complicated things with their code. 

The solution I ended up with is to represent everything as `usizes`
pointing inside a large array holding all values (such an array is
sort of necessary for GC anyway). This defeats a good part of the type
system. Oh well.

## Todo

### Large

* GC
* Be faster and less stupid

### Medium

* `call/cc` (all necessary ingredients should be present)
* Quasiquotation
* Standard library
* Better input handling, read from files, etc.
* Maybe support exact and inexact numbers, and complexes and rationals
 * Catch overflows or support bigints
 * Idea: Make a `Value::Numeric` that would then contain the numeric
   subtypes. 
* Macro support

### Small

* Make `structs` for the more complex values in `Value`, so we can
provide appropriate methods in a type-safe manner.
* Make `define` support the function definition shorthand (could be done
with a macro, maybe?)
* We can store bodies, and probably argument lists as well, as vecs instead
of Scheme lists, as they have well-defined shapes. This does pose a
problem for continuations, because they want a reference inside the
arena.
* Allow fully disabling rustyline [using features](
https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section).


## Useful documentation

* https://github.com/scheme-requests-for-implementation
* [Page on call/cc](http://www.madore.org/~david/computers/callcc.html#sec_whatis)
* https://schemers.org/Documents/Standards/R5RS/HTML/
* https://github.com/ashinn/chibi-scheme/blob/master/tests/r5rs-tests.scm
* https://github.com/kenpratt/rusty_scheme/blob/master/src/interpreter/cps_interpreter.rs
* _Lisp in Small Pieces_
