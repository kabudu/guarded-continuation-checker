# OpenTitan PWM lagged channel-relation probe v1

The source-derived lag-2 hypothesis is negative.

The complete 16-row cohort checked channels 0 and 1 in both temporal
orientations, under equality and difference, at horizons 4, 8, 12 and 16.
Every row has independently verified history coverage at frame 2. Every
relation also has a verified counterexample at that first valid comparison
frame.

| Measure | Result |
| --- | ---: |
| Primary rows | 16 |
| Verified coverage rows | 16 |
| Verified relation counterexamples | 16 |
| SAFE relations | 0 |
| First valid comparison frame | 2 |
| Earliest counterexample frame | 2 |
| History state | 4 bits |
| Certificate bytes per shortest-prefix check | 125 |

Both equality and difference fail at frame 2 because the symbolic firmware
classes can independently alter channel configuration and inversion. A fixed
physical phase delay does not align independently controlled channels into one
functional temporal relation.

The first exploratory run asked the solver for one witness over each complete
horizon. Those witnesses were valid but not necessarily shortest. They were
not retained. The checked-in probe instead searches monotonically from prefix
zero and retains the first verified counterexample, yielding frame 2
consistently for every horizon.

No maintained-baseline or portfolio work is authorized because the
predeclared survival gate failed.
