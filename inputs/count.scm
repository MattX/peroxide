(define count-to
  (lambda (cnt max)
    (if (= cnt max)
      cnt
      (count-to (+ 1 cnt) max))))

(count-to 0 10)
(count-to 0 100)
(count-to 0 1000)
(count-to 0 10000)
(count-to 0 100000)

