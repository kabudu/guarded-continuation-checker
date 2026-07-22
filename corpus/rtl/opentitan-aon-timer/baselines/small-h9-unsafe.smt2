(set-logic QF_BV)
; The reset-free count at frame 9 reaches threshold 9.
(assert (bvuge #x00000009 #x00000009))
(check-sat)
