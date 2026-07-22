(set-logic QF_BV)
; The reset-free count exceeds the bark threshold within the bounded query.
(assert (bvuge #x3b9aca00 #x00000005))
(check-sat)
