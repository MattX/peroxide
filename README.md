# Peroxide

A scheme interpreter in Rust. Aims for R5RS/R7RS compliance. Heavily based
on the interpreter described in _Lisp in Small Pieces_.

## Usage

You can just run `cargo run` to run the interpreter. Some
internal options can be tweaked; try `cargo run -- --help`
for more information.

Set `RUST_LOG=peroxide=debug` or `RUST_LOG=peroxide=trace` to see
debugging information, especially GC-related messages. (This may make the
system very slow.)

## General implementation notes

This is a bytecode compiling implementation: scheme code is first converted to bytecode, then interpreted by a virtual
machine.

The standard library is essentially ripped off [Chibi Scheme](https://github.com/ashinn/chibi-scheme). See
[init.scm](src/scheme-lib/init.scm) for license details. Credit to Alex Shinn for writing it.

Peroxide is strictly single-threaded.

This comes with a very simple garbage collector. See the comment in [heap.rs](src/heap.rs) for implementation details.
Unfortunately it meshes poorly with Rust's memory management. The key thing to remember when making changes,
especially to the AST parser, is that any call to `arena.insert()` (the method used to ask the GC for memory) may
trigger a garbage-collection pass and destroy anything that isn't rooted. Make sure to hold `RootPtr`s to any
Scheme data you care about when doing stuff!

The macro system was another important implementation question. I ended up going with a system similar to Chibi
Scheme's so that I could reuse more of the standard library 🙃. This does mean that, in addition to `syntax-case`,
Peroxide supports the more general syntactic closure macro paradigm. See [doc/macros.md](doc/macros.md) for details.

## Todo

See [todo.md](doc/todo.md) for a list of things to do.

## Useful references

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
* https://www.gnu.org/software/mit-scheme/documentation/stable/mit-scheme-ref.html#Syntactic-Closures
