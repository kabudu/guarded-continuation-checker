(set-logic QF_BV)
; Neither 2,000,000,000 nor 4,000,000,000 is reachable by frame 1,000,000,000.
(assert (or (bvuge #x3b9aca00 #x77359400)
            (bvuge #x3b9aca00 #xee6b2800)))
(check-sat)
