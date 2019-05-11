; Booleans

(define (not x) (if x #f #t))
(define (boolean? x) (if (eq? x #t) #t (eq? x #f)))

; Lists and pairs

(define (caar x) (car (car x)))
(define (cadr x) (car (cdr x)))
(define (cdar x) (cdr (car x)))
(define (cddr x) (cdr (cdr x)))

(define (null? x) (eq? x '()))
(define (list? x) (if (pair? x) #t (null? x)))

(define (list . args) args)

; TODO: replace with internal define
(define (length* ls acc)
  (if (null? x)
      acc
      (length* (cdr ls) (+ 1 acc))))

; TODO: signal errors
(define (length ls)
  (length* ls 0))
