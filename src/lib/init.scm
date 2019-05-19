;; Copyright 2018-2019 Matthieu Felix
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

; Numeric

(define (zero? x) (= 0 x))
(define (positive? x) (> 0 x))
(define (negative? x) (< 0 x))

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
(define (list? x) (if (pair? x) #t (null? x)))

(define (list . args) args)

(define (length ls)
  (define (length* ls acc)
    (if (null? ls)
        acc
        (length* (cdr ls) (+ 1 acc))))
  (length* ls 0))

(define (append2 l1 l2)
  (if (null? l1)
      l2
      (cons (car l1) (append2 (cdr l1) l2))))

(define (appendn lists)
  (if (null? lists)
      '()
      (if (null? (cdr lists))
          (car lists)
          (appendn (list (car lists) (appendn (cdr lists)))))))

(define (append . lists)
  (apply appendn lists))

(define (acc-reverse l acc)
  (if (null? l)
      acc
      (acc-reverse (cdr l) (cons (car l) acc))))

(define (reverse l)
  (acc-reverse l '()))

(define (list-tail l k)
  (if (zero? k)
      x
      (list-tail (cdr x) (- k 1))))

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
