(set-logic QF_BV)
; Maximum reset-add count at frame 8 cannot reach threshold 9.
(assert (bvuge #x00000008 #x00000009))
(check-sat)
