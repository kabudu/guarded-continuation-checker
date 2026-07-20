(set-logic QF_BV)
; Maximum reset-add count at frame 1,000,000,000 is below 4,000,000,000.
(assert (bvuge #x3b9aca00 #xee6b2800))
(check-sat)
