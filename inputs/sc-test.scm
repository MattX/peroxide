(define my-list '())

(let ((x 5))
  (let-syntax ((push
    (sc-macro-transformer
      (begin
        (display x)
        (lambda (exp env)
         (let ((item (make-syntactic-closure env '() (cadr exp)))
               (list (make-syntactic-closure env '() (caddr exp))))
          `(set! ,list (cons ,item ,list))))))))
    (push 5 my-list)))

my-list
