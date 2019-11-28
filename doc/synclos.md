# Syntactic Closures

A lot of this is ripped off from
https://lists.gnu.org/archive/html/chicken-users/2008-04/msg00013.html.


> There are two completely orthogonal aspects of macro systems—whether they are hygienic
> or unhygienic, and whether they are low-level or high-level.

Examples of macro systems with various characteristics:

|            	| Low-level          	| High-level                             	|
|------------	|--------------------	|----------------------------------------	|
| Hygienic   	| Syntactic closures 	| `syntax-rules`                         	|
| Unhygienic 	| `defmacro`         	| [m4](https://www.gnu.org/software/m4/) 	|


## The need for hygiene

There are two separate issues with unhygienic macro systems.

1. When a macro introduces new symbols, they become visible to any code that's transplanted
   from the outer scope, possibly shadowing bindings or at least giving rise to obscure
   errors:
   
   ```scheme
   (defmacro swap! (a b)
     `(let ((value ,var1)) 
        (set! ,var1 ,var2) 
        (set! ,var2 value))) 
   ```
   
   In this case, if one of `a` or `b` is also the symbol `value`, this will generate garbage
   code. This problem is entirely solvable using `gensym`, but annoying.
   
2. A macro's expected bindings might no longer be current where the macro is expanded.
   For instance,
   
   ```scheme
   (let ((set! display))
      (swap! x y))
   ```
   
   Here, `swap!` expected `set!` to be the primitive, but it's some value that the user
   was shadowing.
   
In either case, what we want is for identifiers to be tied to their environment of
definition, not the environment in which they happen to be transplanted by macro expansion.

Note that this is not always true! There are examples of useful non-hygienic macros,
but hygiene is desired most of the time. Scheme macro systems such as syntactic closures
let us control hygiene very finely if needed, so we can get the best of both worlds.

How can we implement hygene? There are several solutions, but Peroxide uses syntactic
closures, which are due to Bawden & Rees. [^1]

[^1]: Bawden, Alan, and Jonathan Rees. Syntactic Closures. No. AI-M-1049. MASSACHUSETTS
      INST OF TECH CAMBRIDGE ARTIFICIAL INTELLIGENCE LAB, 1988.
      https://apps.dtic.mil/dtic/tr/fulltext/u2/a195921.pdf
      
## A low-level macro system

In Peroxide and many other Scheme systems, a macro is a lambda that takes three arguments:
the form to expand, the macro's definition environment, and the macro's expansion
environment. The macro produces code as a result, which will be inserted at the macro
call site.

For instance, you can declare a low-level macro in the following way:

```scheme
(define-syntax unless
  ; Note the characteristic signature
  (lambda (form usage-env macro-env)
    (let ((condition (cadr form))
          (consequent (caddr form)))
      `(if (not ,condition) ,consequent))))  
```

When the compiler sees this declaration, it immediately compiles the lambda, and binds
it to the macro `unless`. Later, code like

```
(unless (> 0 count) (fail))
```

will result in our lambda being called with parameters `(unless (> 0 count) (fail))` (it's
not critically important, but note that the macro name itself is passed to the macro as
part of the form to expand), and two environment objects representing the current and
definition environments.

The lambda outputs `(if (not (> 0 count)) (fail))`. Symbols are treated completely normally,
i.e. they are assumed to refer to bindings that exist in the environment at the use site.

Note too that the lambda itself is compiled in the global environment, and, since lambdas
close over their definition environments, this is also where the execution happens.

Even if the lambda is declared with a local form, such as:

```scheme
(define x 'outer)
(let ((x 'inner))
  (let-syntax ((print-x
                 (lambda (form usage-env macro-env)
                   `(display ',x))))
    (print-x)))
```

~~the lambda itself will run in the global environment.~~ The `print-x` macro defined above,
for
instance, will always produce `(display 'outer)` as a result. However, still in the case
above, `macro-env` will be the environment created by the `let` form. Conversely, in
the case of a macro defined with `define-syntax`, `macro-env` will always be
the global environment.

The reason for this is that macro expansion is interleaved with code compilation, which
precedes code execution. The only environment that we can reasonably hope to exist at
macro expansion time is a global environment—inner environments still don't have any
bytecode to actually create them yet, much less evaluate the values in the environment.

## The syntactic closure primitives

What do these mysterious environment objects look like? And how do we use them to
guarantee hygiene?

There's not much you can do with an environment object, except shove it in a syntactic
closure. A syntactic closure is created using `make-syntactic-closure`:

```scheme
(make-syntactic-closure env free-variables form)
```

The easiest way to use a syntactic closure is on a symbol. Take our `unless` macro as
an example. Its implementation above is vulnerable to shadowing `if` and `not`. We can
rewrite the macro using syntactic closures:

```scheme
(define-syntax unless
  (lambda (form usage-env macro-env)
    (let ((condition (cadr form))
          (consequent (caddr form))
          (renamed-not (make-syntactic-closure macro-env '() 'not))
          (renamed-if (make-syntactic-closure macro-env '() 'if)))
      `(,renamed-if (,renamed-not ,condition) ,consequent))))  
```

(We also need to rename `if`, because it's legal to shadow keywords in Scheme.)

As you might have guessed, the calls to `make-syntactic-closure` here produce symbols
that point to the specified environment (`macro-env`), instead of the current environment.

`make-syntactic-closure` can also be used to make all symbols within a large form point
to a different environment. For instance:

```scheme
(define x 'outer)
(let ((x 'middle))
  (let-syntax ((print-middle-x
                 (lambda (form usage-env macro-env)
                   (make-syntactic-closure macro-env '() '(display x)))))
    (let ((x 'inner) (display #f))
      (print-middle-x))))
```

Overall, this technique lets us precisely control which identifiers should come from
which environment, solving hygiene problem #1.

### Shadowing a symbol-in-a-syntactic-closure

After introducing syntactic closures to a Scheme system, we end up with two kinds of
identifiers: regular old symbols, and symbols in a (possibly nested) syntactic closure,
which I'll call ssc.

All identifiers can be assigned to, and the meaning is straightforward: for an ssc,
we simply edit the memory location pointed to by the binding, like we would for a regular
symbol. The issue of shadowing is more complex. Ignoring syntactic sugar, an identifier
is shadowed by introducing a lambda that uses that identifier as a parameter.

When an ssc is used as a lambda parameter, any references to it within the body of that
lambda will instead refer to that lambda's parameter. Outside the body of the lambda,
the ssc does not change meaning. This lets a macro effectively declare a binding as
private by using a syntactically closed symbol in a lambda argument or a let definition.

Note that two different sscs, even with the same environment and the same closed symbol,
will refer to two different parameters if they are shadowed. For instance:

```scheme
(define x 0)
(define-syntax mymacro
  (lambda (f use-env mac-env)
    (let ((x1 (make-syntactic-closure mac-env '() 'x))
          (x2 (make-syntactic-closure mac-env '() 'x)))
      `(list
         ,(identifier=? mac-env x1 mac-env x2)
         ,(let ((x1 2))
            (identifier=? mac-env x1 mac-env x2))))))

=> (#t #f)
```

In effect, if you need to solve problem 2 by creating a variable that's invisible to
expanded code, you can use an ssc as the target of your `let`. If you're using the ssc
just for that, it also doesn't matter which environment you create the syntactic closure
for.

### Other syntactic closure methods

Of note are also `(identifier? x)`, which returns true iff `x` is either a symbol, or
a (possibly nested) syntactic closure around a symbol, and `(identifier=? ex x ey y)`, which
returns `true` iff `x` and `y` are both identifiers, and they refer to the same binding
when `x` is looked up in `ex` and `y` in `ey`.
Note that this can be true even if `x` and `y` are identifiers in different syntactic
closures, as long as they do refer to the same binding.

Chibi and Peroxide also make a distinction between `syntax-quote`, which is really just
what `quote` is in most other Scheme systems, and `quote`, which behaves like `syntax-quote`
except it will strip (possibly nested) syntactic closures from its argument. The two
are interchangeable in the absence of syntactic closures. 

## Issues with nested macros

Several issues can occur with nested macros.


