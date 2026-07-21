(set-logic QF_BV)
; Neither bark threshold 5 nor bite threshold 9 is reachable at frame 4.
(assert (or (bvuge #x00000004 #x00000005)
            (bvuge #x00000004 #x00000009)))
(check-sat)
