;; Copyright 2018-2020 Matthieu Felix
;;
;; Licensed under the Apache License, Version 2.0 (the "License");
;; you may not use this file except in compliance with the License.
;; You may obtain a copy of the License at
;;
;; https://www.apache.org/licenses/LICENSE-2.0
;;
;; Unless required by applicable law or agreed to in writing, software
;; distributed under the License is distributed on an "AS IS" BASIS,
;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;; See the License for the specific language governing permissions and
;; limitations under the License.
;;
;;
;; Portions of this file are taken from Chibi Scheme.
;; https://github.com/ashinn/chibi-scheme/blob/d0cb74bef464accdc9fef6b67c03de27f00567bb/lib/init-7.scm
;;
;; Chibi Scheme is covered by the following license:
;;
;;   Copyright (c) 2000-2015 Alex Shinn
;;   All rights reserved.
;;   
;;   Redistribution and use in source and binary forms, with or without
;;   modification, are permitted provided that the following conditions
;;   are met:
;;   1. Redistributions of source code must retain the above copyright
;;      notice, this list of conditions and the following disclaimer.
;;   2. Redistributions in binary form must reproduce the above copyright
;;      notice, this list of conditions and the following disclaimer in the
;;      documentation and/or other materials provided with the distribution.
;;   3. The name of the author may not be used to endorse or promote products
;;      derived from this software without specific prior written permission.
;;   
;;   THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
;;   IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
;;   OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
;;   IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
;;   INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
;;   NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
;;   DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
;;   THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
;;   (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
;;   THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
;;

; Numeric

(define (zero? x) (= 0 x))
(define (positive? x) (> x 0))
(define (negative? x) (< x 0))


; Booleans

(define (not x) (if x #f #t))
(define (boolean? x) (if (eq? x #t) #t (eq? x #f)))


; Lists and pairs

(define (caar x) (car (car x)))
(define (cadr x) (car (cdr x)))
(define (cdar x) (cdr (car x)))
(define (cddr x) (cdr (cdr x)))

(define (caaar x) (car (car (car x))))
(define (caadr x) (car (car (cdr x))))
(define (cadar x) (car (cdr (car x))))
(define (caddr x) (car (cdr (cdr x))))
(define (cdaar x) (cdr (car (car x))))
(define (cdadr x) (cdr (car (cdr x))))
(define (cddar x) (cdr (cdr (car x))))
(define (cdddr x) (cdr (cdr (cdr x))))

(define (caaaar x) (car (car (car (car x)))))
(define (caaadr x) (car (car (car (cdr x)))))
(define (caadar x) (car (car (cdr (car x)))))
(define (caaddr x) (car (car (cdr (cdr x)))))
(define (cadaar x) (car (cdr (car (car x)))))
(define (cadadr x) (car (cdr (car (cdr x)))))
(define (caddar x) (car (cdr (cdr (car x)))))
(define (cadddr x) (car (cdr (cdr (cdr x)))))
(define (cdaaar x) (cdr (car (car (car x)))))
(define (cdaadr x) (cdr (car (car (cdr x)))))
(define (cdadar x) (cdr (car (cdr (car x)))))
(define (cdaddr x) (cdr (car (cdr (cdr x)))))
(define (cddaar x) (cdr (cdr (car (car x)))))
(define (cddadr x) (cdr (cdr (car (cdr x)))))
(define (cdddar x) (cdr (cdr (cdr (car x)))))
(define (cddddr x) (cdr (cdr (cdr (cdr x)))))

(define (null? x) (eq? x '()))

; TODO must handle cyclic lists.
(define (list? x)
  (if (pair? x)
      (list? (cdr x))
      (null? x)))

(define (list . args) args)

(define (length ls)
  (define (length* ls acc)
    (if (null? ls)
        acc
        (length* (cdr ls) (+ 1 acc))))
  (length* ls 0))

(define (acc-reverse l acc)
  (if (null? l)
      acc
      (acc-reverse (cdr l) (cons (car l) acc))))

(define (reverse l)
  (acc-reverse l '()))

(define (append2 l res)
  (if (null? l)
      res
      (cons (car l) (append2 (cdr l) res))))

(define (append-helper ls res)
  (if (null? ls)
      res
      (append-helper (cdr ls) (append2 (car ls) res))))

(define (append . o)
  (if (null? o)
      '()
      ((lambda (lol)
         (append-helper (cdr lol) (car lol)))
       (reverse o))))

(define (list-tail l k)
  (if (zero? k)
      l
      (list-tail (cdr l) (- k 1))))

(define (list-ref l k)
  (car (list-tail l k)))

(define (mem predicate obj ls)
  (if (null? ls)
      #f
      (if (predicate obj (car ls))
          ls
          (mem predicate obj (cdr ls)))))

(define (memq obj ls)
  (mem eq? obj ls))
(define (memv obj ls)
  (mem eqv? obj ls))
(define (member obj ls)
  (mem equal? obj ls))

(define (ass predicate obj ls)
  (if (null? ls)
      #f
      (if (predicate obj (caar ls))
          (car ls)
          (ass predicate obj (cdr ls)))))

(define (assq obj ls)
  (ass eq? obj ls))
(define (assv obj ls)
  (ass eqv? obj ls))
(define (assoc obj ls)
  (ass equal? obj ls))


; Control features

(define (any? values)
  (if (null? values)
      #f
      (if (car values)
          #t
          (any? (cdr values)))))

(define (map1acc fn ls acc)
  (if (null? ls)
      (reverse acc)
      (map1acc fn (cdr ls) (cons (fn (car ls)) acc))))

(define (map1 fn ls)
  (map1acc fn ls '()))

(define (mapnacc fn lists acc)
  (if (any? (map1 null? lists))
      (reverse acc)
      (mapnacc fn (map1 cdr lists) (cons (apply fn (map1 car lists)) acc))))

(define (map fn . lists)
  ((lambda (len)
     (if (= 0 len)
         #f
         (if (= 1 len)
             (map1 fn (car lists))
             (mapnacc fn lists '()))))
   (length lists)))

(define (for-each fn . lists)
  (apply map fn lists))

(define (every proc l)
  (if (null? l)
      #t
      (if (proc (car l))
          (every proc (cdr l))
          #f)))

(define (any pred ls . lol)
  (define (any1 pred ls)
    (if (pair? (cdr ls))
        ((lambda (x) (if x x (any1 pred (cdr ls)))) (pred (car ls)))
        (pred (car ls))))
  (define (anyn pred lol)
    (if (every pair? lol)
        ((lambda (x) (if x x (anyn pred (map cdr lol))))
         (apply pred (map car lol)))
        #f))
  (if (null? lol) (if (pair? ls) (any1 pred ls) #f) (anyn pred (cons ls lol))))

(define (list->vector l)
  (define (list->vector l v k)
    (if (null? l)
        v
        (begin
          (vector-set! v k (car l))
          (list->vector (cdr l) v (+ k 1)))))
  ((lambda (v)
     (list->vector l v 0))
   (make-vector (length l))))

(define (vector->list v)
  (define (vector->list v l k)
    (if (= k -1)
        l
        (vector->list v (cons (vector-ref v k) l) (- k 1))))
  (vector->list v '() (- (vector-length v) 1)))

; Characters

(define (char=? . c)
  (apply = (map char->integer c)))
(define (char<? . c)
  (apply < (map char->integer c)))
(define (char>? . c)
  (apply > (map char->integer c)))
(define (char<=? . c)
  (apply <= (map char->integer c)))
(define (char>=? . c)
  (apply >= (map char->integer c)))

(define (char-ci=? . c)
  (apply char=? (map char-downcase c)))
(define (char-ci<? . c)
  (apply char<? (map char-downcase c)))
(define (char-ci>? . c)
  (apply char>? (map char-downcase c)))
(define (char-ci<=? . c)
  (apply char<=? (map char-downcase c)))
(define (char-ci>=? . c)
  (apply char>=? (map char-downcase c)))


;; Syntax

(define sc-macro-transformer
  (lambda (f)
    (lambda (expr use-env mac-env)
      (make-syntactic-closure mac-env '() (f expr use-env)))))

(define rsc-macro-transformer
  (lambda (f)
    (lambda (expr use-env mac-env)
      (f expr mac-env))))

(define er-macro-transformer
  (lambda (f)
    (lambda (expr use-env mac-env)
      ((lambda (rename compare) (f expr rename compare))
       ((lambda (renames)
          (lambda (identifier)
            ((lambda (cell)
               (if cell
                   (cdr cell)
                   ((lambda (name)
                      (set! renames (cons (cons identifier name) renames))
                      name)
                    (make-syntactic-closure mac-env '() identifier))))
             (assq identifier renames))))
        '())
       (lambda (x y) (identifier=? use-env x use-env y))))))

(define-syntax cond
  (er-macro-transformer
   (lambda (expr rename compare)
     (if (null? (cdr expr))
         (if #f #f)
         ((lambda (cl)
            (if (compare (rename 'else) (car cl))
                (if (pair? (cddr expr))
                    (error "non-final else in cond" expr)
                    (cons (rename 'begin) (cdr cl)))
                (if (if (null? (cdr cl)) #t (compare (rename '=>) (cadr cl)))
                    (list (list (rename 'lambda) (list (rename 'tmp))
                                (list (rename 'if) (rename 'tmp)
                                      (if (null? (cdr cl))
                                          (rename 'tmp)
                                          (list (car (cddr cl)) (rename 'tmp)))
                                      (cons (rename 'cond) (cddr expr))))
                          (car cl))
                    (list (rename 'if)
                          (car cl)
                          (cons (rename 'begin) (cdr cl))
                          (cons (rename 'cond) (cddr expr))))))
          (cadr expr))))))


(define-syntax or
  (er-macro-transformer
   (lambda (expr rename compare)
     (cond ((null? (cdr expr)) #f)
           ((null? (cddr expr)) (cadr expr))
           (else
            (list (rename 'let) (list (list (rename 'tmp) (cadr expr)))
                  (list (rename 'if) (rename 'tmp)
                        (rename 'tmp)
                        (cons (rename 'or) (cddr expr)))))))))

(define-syntax and
  (er-macro-transformer
   (lambda (expr rename compare)
     (cond ((null? (cdr expr)))
           ((null? (cddr expr)) (cadr expr))
           (else (list (rename 'if) (cadr expr)
                       (cons (rename 'and) (cddr expr))
                       #f))))))

(define-syntax quasiquote
  (er-macro-transformer
   (lambda (expr rename compare)
     (define (qq x d)
       (cond
        ((pair? x)
         (cond
          ((compare (rename 'unquote) (car x))
           (if (<= d 0)
               (cadr x)
               (list (rename 'list) (list (rename 'quote) 'unquote)
                     (qq (cadr x) (- d 1)))))
          ((compare (rename 'unquote-splicing) (car x))
           (if (<= d 0)
               (list (rename 'cons) (qq (car x) d) (qq (cdr x) d))
               (list (rename 'list) (list (rename 'quote) 'unquote-splicing)
                     (qq (cadr x) (- d 1)))))
          ((compare (rename 'quasiquote) (car x))
           (list (rename 'list) (list (rename 'quote) 'quasiquote)
                 (qq (cadr x) (+ d 1))))
          ((and (<= d 0) (pair? (car x))
                (compare (rename 'unquote-splicing) (caar x)))
           (if (null? (cdr x))
               (cadr (car x))
               (list (rename 'append) (cadr (car x)) (qq (cdr x) d))))
          (else
           (list (rename 'cons) (qq (car x) d) (qq (cdr x) d)))))
        ((vector? x) (list (rename 'list->vector) (qq (vector->list x) d)))
        ((if (identifier? x) #t (null? x)) (list (rename 'quote) x))
        (else x)))
     (qq (cadr expr) 0))))

(define-syntax letrec
  (er-macro-transformer
   (lambda (expr rename compare)
     ((lambda (defs)
        `((,(rename 'lambda) () ,@defs ,@(cddr expr))))
      (map (lambda (x) (cons (rename 'define) x)) (cadr expr))))))

(define-syntax let
  (er-macro-transformer
   (lambda (expr rename compare)
     (if (null? (cdr expr)) (error "empty let" expr))
     (if (null? (cddr expr)) (error "no let body" expr))
     ((lambda (bindings)
        (if (list? bindings) #f (error "bad let bindings"))
        (if (every (lambda (x)
                     (if (pair? x) (if (pair? (cdr x)) (null? (cddr x)) #f) #f))
                   bindings)
            ((lambda (vars vals)
               (if (identifier? (cadr expr))
                   `((,(rename 'lambda) ,vars
                      (,(rename 'letrec) ((,(cadr expr)
                                           (,(rename 'lambda) ,vars
                                            ,@(cdr (cddr expr)))))
                       (,(cadr expr) ,@vars)))
                     ,@vals)
                   `((,(rename 'lambda) ,vars ,@(cddr expr)) ,@vals)))
             (map car bindings)
             (map cadr bindings))
            (error "bad let syntax" expr)))
      (if (identifier? (cadr expr)) (car (cddr expr)) (cadr expr))))))

(define-syntax let*
  (er-macro-transformer
   (lambda (expr rename compare)
     (if (null? (cdr expr)) (error "empty let*" expr))
     (if (null? (cddr expr)) (error "no let* body" expr))
     (if (null? (cadr expr))
         `(,(rename 'let) () ,@(cddr expr))
         (if (if (list? (cadr expr))
                 (every
                  (lambda (x)
                    (if (pair? x) (if (pair? (cdr x)) (null? (cddr x)) #f) #f))
                  (cadr expr))
                 #f)
             `(,(rename 'let) (,(caar (cdr expr)))
               (,(rename 'let*) ,(cdar (cdr expr)) ,@(cddr expr)))
              (error "bad let* syntax"))))))


(define-syntax case
  (er-macro-transformer
   (lambda (expr rename compare)
     (define (body exprs)
       (cond
        ((null? exprs)
         (rename 'tmp))
        ((compare (rename '=>) (car exprs))
         `(,(cadr exprs) ,(rename 'tmp)))
        (else
         `(,(rename 'begin) ,@exprs))))
     (define (clause ls)
       (cond
        ((null? ls) #f)
        ((compare (rename 'else) (caar ls))
         (body (cdar ls)))
        ((and (pair? (car (car ls))) (null? (cdr (car (car ls)))))
         `(,(rename 'if) (,(rename 'eqv?) ,(rename 'tmp)
                          (,(rename 'quote) ,(car (caar ls))))
           ,(body (cdar ls))
           ,(clause (cdr ls))))
        (else
         `(,(rename 'if) (,(rename 'memv) ,(rename 'tmp)
                          (,(rename 'quote) ,(caar ls)))
           ,(body (cdar ls))
           ,(clause (cdr ls))))))
     `(let ((,(rename 'tmp) ,(cadr expr)))
        ,(clause (cddr expr))))))

(define-syntax do
  (er-macro-transformer
   (lambda (expr rename compare)
     (let* ((body
             `(,(rename 'begin)
               ,@(cdr (cddr expr))
               (,(rename 'lp)
                ,@(map (lambda (x)
                         (if (pair? (cddr x))
                             (if (pair? (cdr (cddr x)))
                                 (error "too many forms in do iterator" x)
                                 (car (cddr x)))
                             (car x)))
                       (cadr expr)))))
            (check (car (cddr expr)))
            (wrap
             (if (null? (cdr check))
                 `(,(rename 'let) ((,(rename 'tmp) ,(car check)))
                   (,(rename 'if) ,(rename 'tmp)
                    ,(rename 'tmp)
                    ,body))
                 `(,(rename 'if) ,(car check)
                   (,(rename 'begin) ,@(cdr check))
                   ,body))))
       `(,(rename 'let) ,(rename 'lp)
         ,(map (lambda (x) (list (car x) (cadr x))) (cadr expr))
         ,wrap)))))

(define-syntax delay-force
  (er-macro-transformer
   (lambda (expr rename compare)
     `(,(rename 'promise) #f (,(rename 'lambda) () ,(cadr expr))))))

(define-syntax delay
  (er-macro-transformer
   (lambda (expr rename compare)
     `(,(rename 'delay-force) (,(rename 'promise) #t ,(cadr expr))))))

(define-syntax define-auxiliary-syntax
  (er-macro-transformer
   (lambda (expr rename compare)
     `(,(rename 'define-syntax) ,(cadr expr)
       (,(rename 'er-macro-transformer)
        (,(rename 'lambda) (expr rename compare)
         (,(rename 'error) "invalid use of auxiliary syntax" ',(cadr expr))))))))

(define-auxiliary-syntax _)
(define-auxiliary-syntax =>)
(define-auxiliary-syntax ...)
(define-auxiliary-syntax else)
(define-auxiliary-syntax unquote)
(define-auxiliary-syntax unquote-splicing)

;;;

(define (find-tail pred ls)
  (and (pair? ls) (if (pred (car ls)) ls (find-tail pred (cdr ls)))))

(define (find pred ls)
  (cond ((find-tail pred ls) => car) (else #f)))

(define (strip-syntactic-closures expr)
  (if (syntactic-closure? expr)
      (strip-syntactic-closures (syntactic-closure-expression expr))
      expr))

(define (identifier->symbol id)
  (let ((stripped (strip-syntactic-closures id)))
     (if (symbol? id)
         id
         (error "Not an identifier" id))))


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;; syntax-rules

(define (syntax-rules-transformer expr rename compare)
  (let ((ellipsis-specified? (identifier? (cadr expr)))
        (_er-macro-transformer (rename 'er-macro-transformer))
        (_lambda (rename 'lambda))      (_let (rename 'let))
        (_begin (rename 'begin))        (_if (rename 'if))
        (_and (rename 'and))            (_or (rename 'or))
        (_eq? (rename 'eq?))            (_equal? (rename 'equal?))
        (_car (rename 'car))            (_cdr (rename 'cdr))
        (_cons (rename 'cons))          (_pair? (rename 'pair?))
        (_null? (rename 'null?))        (_expr (rename 'expr))
        (_rename (rename 'rename))      (_compare (rename 'compare))
        (_quote (rename 'syntax-quote)) (_apply (rename 'apply))
        (_append (rename 'append))      (_map (rename 'map))
        (_vector? (rename 'vector?))    (_list? (rename 'list?))
        (_len (rename 'len))            (_length (rename 'length))
        (_- (rename '-))   (_>= (rename '>=))   (_error (rename 'error))
        (_ls (rename 'ls)) (_res (rename 'res)) (_i (rename 'i))
        (_reverse (rename 'reverse))
        (_vector->list (rename 'vector->list))
        (_list->vector (rename 'list->vector)))
    (define ellipsis (if ellipsis-specified? (cadr expr) (rename '...)))
    (define lits (if ellipsis-specified? (car (cddr expr)) (cadr expr)))
    (define forms (if ellipsis-specified? (cdr (cddr expr)) (cddr expr)))
    (define (expand-pattern pat tmpl)
      (let lp ((p (cdr pat))
               (x (list _cdr _expr))
               (dim 0)
               (vars '())
               (k (lambda (vars)
                    (list _cons (expand-template tmpl vars) #f))))
        (let ((v (gensym "v.")))
          (list
           _let (list (list v x))
           (cond
            ((identifier? p)
             (if (any (lambda (l) (compare p l)) lits)
                 (list _and
                       (list _compare v (list _rename (list _quote p)))
                       (k vars))
                 (list _let (list (list p v)) (k (cons (cons p dim) vars)))))
            ((ellipsis? p)
             (cond
              ((not (null? (cdr (cdr p))))
               (cond
                ((any (lambda (x) (and (identifier? x) (compare x ellipsis)))
                      (cddr p))
                 (error "multiple ellipses" p))
                (else
                 (let ((len (length (cdr (cdr p))))
                       (_lp (gensym "lp.")))
                   `(,_let ((,_len (,_length ,v)))
                      (,_and (,_>= ,_len ,len)
                             (,_let ,_lp ((,_ls ,v)
                                          (,_i (,_- ,_len ,len))
                                          (,_res (,_quote ())))
                                    (,_if (,_>= 0 ,_i)
                                        ,(lp `(,(cddr p)
                                               (,(car p) ,(car (cdr p))))
                                             `(,_cons ,_ls
                                                      (,_cons (,_reverse ,_res)
                                                              (,_quote ())))
                                             dim
                                             vars
                                             k)
                                        (,_lp (,_cdr ,_ls)
                                              (,_- ,_i 1)
                                              (,_cons (,_car ,_ls)
                                                       ,_res))))))))))
              ((identifier? (car p))
               (list _and (list _list? v)
                     (list _let (list (list (car p) v))
                           (k (cons (cons (car p) (+ 1 dim)) vars)))))
              (else
               (let* ((w (gensym "w."))
                      (_lp (gensym "lp."))
                      (new-vars (all-vars (car p) (+ dim 1)))
                      (ls-vars (map (lambda (x)
                                      (gensym
                                        (symbol->string
                                         (identifier->symbol (car x)))))
                                    new-vars))
                      (once
                       (lp (car p) (list _car w) (+ dim 1) '()
                           (lambda (_)
                             (cons
                              _lp
                              (cons
                               (list _cdr w)
                               (map (lambda (x l)
                                      (list _cons (car x) l))
                                    new-vars
                                    ls-vars)))))))
                 (list
                  _let
                  _lp (cons (list w v)
                            (map (lambda (x) (list x (list _quote '()))) ls-vars))
                  (list _if (list _null? w)
                        (list _let (map (lambda (x l)
                                          (list (car x) (list _reverse l)))
                                        new-vars
                                        ls-vars)
                              (k (append new-vars vars)))
                        (list _and (list _pair? w) once)))))))
            ((pair? p)
             (list _and (list _pair? v)
                   (lp (car p)
                       (list _car v)
                       dim
                       vars
                       (lambda (vars)
                         (lp (cdr p) (list _cdr v) dim vars k)))))
            ((vector? p)
             (list _and
                   (list _vector? v)
                   (lp (vector->list p) (list _vector->list v) dim vars k)))
            ((null? p) (list _and (list _null? v) (k vars)))
            (else (list _and (list _equal? v p) (k vars))))))))
    (define (ellipsis-escape? x) (and (pair? x) (compare ellipsis (car x))))
    (define (ellipsis? x)
      (and (pair? x) (pair? (cdr x)) (compare ellipsis (cadr x))))
    (define (ellipsis-depth x)
      (if (ellipsis? x)
          (+ 1 (ellipsis-depth (cdr x)))
          0))
    (define (ellipsis-tail x)
      (if (ellipsis? x)
          (ellipsis-tail (cdr x))
          (cdr x)))
    (define (all-vars x dim)
      (let lp ((x x) (dim dim) (vars '()))
        (cond ((identifier? x)
               (if (any (lambda (lit) (compare x lit)) lits)
                   vars
                   (cons (cons x dim) vars)))
              ((ellipsis? x) (lp (car x) (+ dim 1) (lp (cddr x) dim vars)))
              ((pair? x) (lp (car x) dim (lp (cdr x) dim vars)))
              ((vector? x) (lp (vector->list x) dim vars))
              (else vars))))
    (define (free-vars x vars dim)
      (let lp ((x x) (free '()))
        (cond
         ((identifier? x)
          (if (and (not (memq x free))
                   (cond ((assq x vars) => (lambda (cell) (>= (cdr cell) dim)))
                         (else #f)))
              (cons x free)
              free))
         ((pair? x) (lp (car x) (lp (cdr x) free)))
         ((vector? x) (lp (vector->list x) free))
         (else free))))
    (define (expand-template tmpl vars)
      (let lp ((t tmpl) (dim 0))
        (cond
         ((identifier? t)
          (cond
           ((find (lambda (v) (eq? t (car v))) vars)
            => (lambda (cell)
                 (if (<= (cdr cell) dim)
                     t
                     (error "too few ...'s"))))
           (else
            (list _rename (list _quote t)))))
         ((pair? t)
          (cond
           ((ellipsis-escape? t)
            (list _quote
                  (if (pair? (cdr t))
                      (if (pair? (cddr t)) (cddr t) (cadr t))
                      (cdr t))))
           ((ellipsis? t)
            (let* ((depth (ellipsis-depth t))
                   (ell-dim (+ dim depth))
                   (ell-vars (free-vars (car t) vars ell-dim)))
              (cond
               ((null? ell-vars)
                (error "too many ...'s"))
               ((and (null? (cdr (cdr t))) (identifier? (car t)))
                ;; shortcut for (var ...)
                (lp (car t) ell-dim))
               (else
                (let* ((once (lp (car t) ell-dim))
                       (nest (if (and (null? (cdr ell-vars))
                                      (identifier? once)
                                      (eq? once (car vars)))
                                 once ;; shortcut
                                 (cons _map
                                       (cons (list _lambda ell-vars once)
                                             ell-vars))))
                       (many (do ((d depth (- d 1))
                                  (many nest
                                        (list _apply _append many)))
                                 ((= d 1) many))))
                  (if (null? (ellipsis-tail t))
                      many ;; shortcut
                      (list _append many (lp (ellipsis-tail t) dim))))))))
           (else (list _cons (lp (car t) dim) (lp (cdr t) dim)))))
         ((vector? t) (list _list->vector (lp (vector->list t) dim)))
         ((null? t) (list _quote '()))
         (else t))))
    (list
     _er-macro-transformer
     (list _lambda (list _expr _rename _compare)
           (list
            _car
            (cons
             _or
             (append
              (map
               (lambda (clause) (expand-pattern (car clause) (cadr clause)))
               forms)
              (list
               (list _cons
                     (list _error "no expansion for"
                           (list (rename 'strip-syntactic-closures) _expr))
                     #f)))))))))

(define-syntax syntax-rules/aux
  (er-macro-transformer syntax-rules-transformer))

(define-syntax syntax-rules
  (er-macro-transformer
   (lambda (expr rename compare)
     (if (identifier? (cadr expr))
         (list (rename 'let) (list (list (cadr expr) #t))
               (cons (rename 'syntax-rules/aux) (cdr expr)))
         (syntax-rules-transformer expr rename compare)))))


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;; values

(define *values-tag* (list 'values))

(define (%values ls)
  (if (and (pair? ls) (null? (cdr ls)))
      (car ls)
      (cons *values-tag* ls)))

(define (values . ls) (%values ls))

(define (call-with-values producer consumer)
  (let ((res (producer)))
    (if (and (pair? res) (eq? *values-tag* (car res)))
        (apply consumer (cdr res))
        (consumer res))))

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;; dynamic-wind

(define (vector . v)
  (list->vector v))

(define %make-point vector)
(define (%point-depth point) (vector-ref point 0))
(define (%point-in point) (vector-ref point 1))
(define (%point-out point) (vector-ref point 2))
(define (%point-parent point) (vector-ref point 3))

(define root-point			; Shared among all state spaces
  (%make-point 0
	      (lambda () (error "winding in to root!"))
	      (lambda () (error "winding out of root!"))
	      #f))

(define %dk
  (let ((dk root-point))
    (lambda o (if (pair? o) (set! dk (car o)) dk))))

(%dk root-point)

(define (dynamic-wind in body out)
  (in)
  (let ((here (%dk)))
    (%dk (%make-point (+ (%point-depth here) 1)
                     in
                     out
                     here))
    (let ((res (body)))
      (%dk here)
      (out)
      res)))

(define (travel-to-point! here target)
  (cond
   ((eq? here target)
    'done)
   ((< (%point-depth here) (%point-depth target))
    (travel-to-point! here (%point-parent target))
    ((%point-in target)))
   (else
    ((%point-out here))
    (travel-to-point! (%point-parent here) target))))

(define (continuation->procedure cont point)
  (lambda res
    (travel-to-point! (%dk) point)
    (%dk point)
    (cont (%values res))))

(define (call-with-current-continuation proc)
  (%call/cc
   (lambda (cont)
     (proc (continuation->procedure cont (%dk))))))

(define call/cc call-with-current-continuation)


;;; Random numeric stuff

(define (abs x)
  (if (negative? x) (- x) x))

(define (inexact? x) (not (exact? x)))

(define exact->inexact inexact)

(define (max x . rest)
  (define (~max hi ls)
    (if (null? ls)
        (exact->inexact hi)
        (~max (if (> (car ls) hi) (car ls) hi) (cdr ls))))
  (if (inexact? x)
      (~max x rest)
      (let lp ((hi x) (ls rest))
        (cond ((null? ls) hi)
              ((inexact? (car ls)) (~max hi ls))
              (else (lp (if (> (car ls) hi) (car ls) hi) (cdr ls)))))))

(define (min x . rest)
  (define (~min lo ls)
    (if (null? ls)
        (exact->inexact lo)
        (~min (if (< (car ls) lo) (car ls) lo) (cdr ls))))
  (if (inexact? x)
      (~min x rest)
      (let lp ((lo x) (ls rest))
        (cond ((null? ls) lo)
              ((inexact? (car ls)) (~min lo ls))
              (else (lp (if (< (car ls) lo) (car ls) lo) (cdr ls)))))))

;; Ports

(define (close-output-port p) (close-port p))

(define (call-with-output-string proc)
  (let ((out (open-output-string)))
    (proc out)
    (let ((res (get-output-string out)))
      (close-output-port out)
      res)))


;; Eval

(define (scheme-report-environment v)
  (if (= v 5) "r5rs" (raise "invalid environment descriptor")))

(define (null-environment v)
  (if (= v 5) "null" (raise "invalid environment descriptor")))
