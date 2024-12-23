(provide dynamic-wind
         (for-syntax parameterize)
         call/cc
         call-with-current-continuation
         make-parameter
         continuation?)

;;;;;; Parameters ;;;;;

(struct Parameter (getter value)
  #:mutable
  #:printer (lambda (obj printer-function) (simple-display "<procedure:parameter-procedure>"))
  #:prop:procedure 0)

(define (make-parameter value)
  (define param (Parameter 'uninitialized value))

  (set-Parameter-getter! param
                         (case-lambda
                           [() (Parameter-value param)]
                           [(new-value) (set-Parameter-value! param new-value)]))

  param)

(define-syntax parameterize
  (syntax-rules ()
    [(parameterize ()
       body ...)
     (begin
       body ...)]

    [(parameterize ([var val]
                    rest ...)
       body ...)

     (let ([old-value (var)])

       (dynamic-wind (lambda () (set-Parameter-value! var val))
                     (lambda ()
                       (parameterize (rest ...)
                         body ...))
                     (lambda () (set-Parameter-value! var old-value))))]))

;;;;;;; Bootstrapping printing functions for various primitive structs

(provide current-input-port
         current-output-port
         current-error-port
         simple-display
         simple-displayln
         newline
         write-char
         write
         write-string)

(define current-input-port (make-parameter (#%default-input-port)))
(define current-output-port (make-parameter (#%default-output-port)))
(define current-error-port (make-parameter (#%default-error-port)))

(define (simple-display x)
  (#%raw-write-string x (current-output-port)))

(define write-string #%raw-write-string)

(define newline
  (case-lambda
    [() (#%raw-write-char #\newline (current-output-port))]
    [(port) (#%raw-write-char #\newline port)]))

(define (simple-displayln x)
  (simple-display x)
  (newline))

(define write-char #%raw-write-char)

(define write
  (case-lambda
    [(arg) (#%raw-write arg (current-output-port))]
    [(arg port) (#%raw-write arg port)]))

;;;;;;;;;;;;;;;;;;;;; Port functions ;;;;;;;;;;;;;;;;;;;;;

(provide call-with-output-string
         with-output-to-string)

(define (call-with-output-string proc)
  (define output-string (open-output-string))
  (proc output-string)
  (get-output-string output-string))

(define (with-output-to-string proc)
  (call-with-output-string (lambda (p)
                             (parameterize ([current-output-port p])
                               (proc)))))

;;;;;;;;;;;;;;;;;;;;; Dynamic Wind ;;;;;;;;;;;;;;;;;;;;;;;

; (define winders '())
(define winders (make-tls '()))

; (define list-tail drop)

; (struct Pair (left right))

(define common-tail
  (lambda (x y)
    (let ([lx (length x)]
          [ly (length y)])
      (let loop ([x (if (> lx ly) (list-tail x (- lx ly)) x)]
                 [y (if (> ly lx) (list-tail y (- ly lx)) y)])

        (if (equal? x y) x (loop (cdr x) (cdr y)))))))

(define do-wind
  (lambda (new)
    (let ([tail (common-tail new (get-tls winders))])
      (let f ([ls (get-tls winders)])
        (when (not (equal? ls tail))
          (begin
            ;; TODO: This is probably wrong!
            ; (displayln "setting winders first" ls)
            (set-tls! winders (cdr ls))
            ((cdr (car ls)))
            (f (cdr ls)))))
      (let f ([ls new])
        (when (not (equal? ls tail))
          (begin
            ; (displayln "setting winders second" ls)
            (f (cdr ls))
            ((car (car ls)))
            (set-tls! winders ls)))))))

(struct Continuation (func)
  #:prop:procedure 0
  #:printer (lambda (obj printer)

              (simple-display "#<procedure>")))

(define call/cc
  (lambda (f)
    (#%prim.call/cc (lambda (k)
                      (f (let ([save (get-tls winders)])
                           (Continuation (lambda (x)
                                           (unless (eq? save (get-tls winders))
                                             (do-wind save))
                                           (k x)))))))))

(define call-with-current-continuation call/cc)

(define (continuation? x)
  (or (Continuation? x) (#%prim.continuation? x)))

(define dynamic-wind
  (lambda (in body out)
    (in)
    (set-tls! winders (cons (cons in out) (get-tls winders)))
    (let ([ans* (call-with-exception-handler (lambda (err)
                                               ;; Catch the exception on the way out

                                               ; (displayln winders)

                                               ; (displayln "catching exception here")

                                               (set-tls! winders (cdr (get-tls winders)))
                                               (out)
                                               (raise-error err)

                                               void)
                                             (lambda () (body)))])

      ; (displayln winders)

      (set-tls! winders (cdr (get-tls winders)))
      (out)
      ans*)))
