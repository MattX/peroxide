* Revise this todo list.

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
