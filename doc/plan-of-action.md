Stuff that needs to be done:

1. Move symbol resolution to the parser stage
    1. Actually, figure out the relationships and responsabilities around
       the environment should be between the AST generator and the compiler.
2. Properly implement the immutable / global / local distinction
3. Implement apply and eval
4. Figure out what the deal is with syntaxic closures used in lambdas
5. Implement that deal
6. Implement call/cc (oh no)
7. Hook up GC

### Re environment maintainer

* The AST generator needs to know about macros (which can be shadowed by
variables) and about any shadowing of special forms. This basically
implies maintaining the full state of the environment.
* The AST generator also needs to know about environments in order
to pass them to macros.
* The AST generator can (must?) also serve as a first pass to detect
implicit variables.
* OTOH the environment also stores whether a variable is definitely 
defined or not, and that only makes sense at the compiler level.

Proposition:

The AST builds the environments, leaving all variables as possibly
defined (basically just ignoring that aspect). Macros get passed