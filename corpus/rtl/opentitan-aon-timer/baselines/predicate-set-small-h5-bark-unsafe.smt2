(set-logic QF_BV)
; The reset-free count reaches the bark threshold at frame 5.
(assert (bvuge #x00000005 #x00000005))
(check-sat)
