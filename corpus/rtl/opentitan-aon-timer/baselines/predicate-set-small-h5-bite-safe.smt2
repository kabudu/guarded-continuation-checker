(set-logic QF_BV)
; The maximum reset-add count at frame 5 remains below bite threshold 9.
(assert (bvuge #x00000005 #x00000009))
(check-sat)
