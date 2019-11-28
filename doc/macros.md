# Macro systems in Scheme

[Good general intro](http://community.schemewiki.org/?scheme-faq-macros)

[Other general post](http://lambda-the-ultimate.org/node/2753), much more in-depth

* We can use homoiconicity for non-hygienic Lisp-like macros: functions that run at expansion / parse time, take in a list and return a list that will be parsed as code.
* Unfortunately that doesn't work for hygienic macro, because there is now a need to tell the parser about more than just raw symbols. Scope must be somehow attached to the symbols.
  * `gensym` or some other restrictions can be used to solve some of these issues but are not very nice. [discussion](http://community.schemewiki.org/?hygiene-versus-gensym)
* Scheme has a native hygienic macro system based on pattern-matching, `syntax-rules`. It's nice and all, but can't express non-hygienic macros and even for hygienic ones it's not always trivial to use because it's purely pattern matching instead of allowing arbitrary code to be run to generate the resulting macro
* Typically speaking, `syntax-rules` is implemented on top of a lower level system [short discussion on HN](https://news.ycombinator.com/item?id=18555658). There seem to be three of them in common use:
  * `syntax-case`, also hygienic (unclear to me if totally or not), complex to implement, `syntax-rules` is easy to define on top of it (it's basically a subset). Officially endorsed by R6RS, which nobody cared about, and which was superseded by R7RS which removed it.
     * [Used by Guile (page is for end users)](https://www.gnu.org/software/guile/manual/html_node/Syntax-Case.html#Syntax-Case).
     * [A more detailed user guide](https://cs.indiana.edu/~dyb/pubs/tr356.pdf)
     * [Long paper about implementation](https://cs.indiana.edu/~dyb/pubs/LaSC-5-4-pp295-326.pdf).
     * [A portable implementation](https://web.archive.org/web/20091021061917/http://ikarus-scheme.org/r6rs-libraries/) (on top of R5RS + other primitives)? which I haven't looked at.
  * Syntactic closures, used in Chibi and MIT scheme
     * [schemewiki page on syntactic closures](http://community.schemewiki.org/?syntactic-closures) 
     * [Used in MIT scheme](https://www.gnu.org/software/mit-scheme/documentation/mit-scheme-ref/Syntactic-Closures.html#Syntactic-Closures). [Here's the implementation of `syntax-rules` on top of it.](https://git.savannah.gnu.org/cgit/mit-scheme.git/tree/src/runtime/syntax-rules.scm)
     * [Chibi implements methods like those of MIT scheme (`er-macro-transformer`, `sc-macro-transformer`, `rsc-macro-transformer`) here](https://github.com/ashinn/chibi-scheme/blob/master/lib/init-7.scm#L147). ([simpler, older version](https://github.com/ashinn/chibi-scheme/blob/2922ed591d1c0dc3be7a92e211ac7b18aa12edcc/lib/init-7.scm#L100)).
     * [Chibi implements `syntax-rules` here](https://github.com/ashinn/chibi-scheme/blob/master/lib/init-7.scm#L863).
     * [Chibi macro system doc](http://synthcode.com/scheme/chibi/#h3_MacroSystem)
  * Implicit/explicit renaming, used in R4RS and Chicken
     * implicit: [In R4RS](https://people.csail.mit.edu/jaffer/r4rs_12.html#SEC77), it's the suggested low-level implementation.
     * explicit: [Short paper by an R4RS author](https://3e8.org/pub/scheme/doc/lisp-pointers/v4i4/p25-clinger.pdf). Also formerly [used in Chicken](https://wiki.call-cc.org/man/4/Macros).
* It is a bit unclear which of these three are more powerful than others. It is apparently possible to implement `syntax-case` on top syntactic closures (+ some helpers?), [as Chibi does](https://github.com/ashinn/chibi-scheme/blob/master/lib/chibi/syntax-case.scm), but it's a bit complicated and apparently needed to add some stuff to the interpreter. [Git PR with discussion](https://github.com/ashinn/chibi-scheme/pull/496). Many systems which claim (and I believe them) to use syntactic closures end up with functions very much like those described in renaming systems.
* There are also pure-syntax-rules implementations. [This one](https://web.archive.org/web/20171216165612/petrofsky.org/src/alexpander.scm) takes in a scheme program with macros and returns one without macros, and implements syntax-rules only.