# Peroxide

A scheme interpreter in Rust. Aims for R5RS/R7RS compliance. Heavily based
on the interpreter described in _Lisp in Small Pieces_.

## General implementation notes

This is a bytecode compiling implementation: scheme code is for converted to bytecode, then interpreted by a virtual
machine. (Right now, the bytecode is not *byte*code in the proper sense as instructions are not 1-byte long, but
this should be fixed in the future).

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

(See also the various TODOs sprinkled through the code)

* ~~Add apply and eval~~
* Implement error handling
  * It can be handled mostly in userspace, but that creates
    extra trickiness around throwing errors from primitives.
* Make sure that syncloses aren't moved outside their domain of validity
* Fix the checked vs unchecked references
* Implement the rest of the stdlib, esp. ports and bytevecs
* Implement libraries
* Allow define-syntax in internal defines
* Implement let optimization
* Inline primitives
* ~~Implement name lookup on error~~ ⇒ needs to be fixed
* Assign names to lambdas when possible
* Keep track of which lines map to which tokens, which map to which
expressions, which map to what bytecode. This will let us have
much better error messages.
* Loooots of code cleanup necessary
  * Remove anything to do with ValRef and many things to do with Arena
  * I think ideally Arena would be used for inserting only, which would also make it easier to see where
    values need to be rooted / protected.
* Figure out how to embed init.scm in a reasonable way (probably by precompiling)
* ~~Allow garbage collection of code like in Python~~
* ~~Handle interrupts~~
* Code blocks can (probably?) be refcounted instead of GCd
* ~~Disallow '%' in symbols after initialization is done?~~ 
* Allow fully disabling rustyline [using features](
https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section).
* Make errors not be strings :)
* Allow meta-commands like `,exit` or `,decompile`
* PoolPtr / Values should only live as long as the Heap or Arena, not forever

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
