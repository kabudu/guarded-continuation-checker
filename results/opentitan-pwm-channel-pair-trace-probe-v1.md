# OpenTitan PWM channel-pair trace probe v1

The fixed six-channel mechanism cohort contains 24 ordered pair queries over
the independently verified structural class `[0, 2, 4]`. It combines equality
and difference relations with a frame-zero control, a two-frame transition,
and constant-low and constant-high two-frame patterns through horizon 2.

The exact retained result is:

| Logical queries | Members | Structural members | Reused queries | Structural bytes | Member evidence | SAFE | UNSAFE |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 24 | 8 | 8 | 16 | 460 B | 8 B | 15 | 9 |

The retained CSV has SHA-256
`172f7c607a4139d6884319f233597b2fdc3b89a264cb3a8d402aec87e1cd5c5a`.

The checker independently rederives the source-bound channel class. Because
both endpoints belong to that same class, equality is universally true and
difference is universally false under every shared input sequence. The eight
one-byte members bind that derived constant. Every UNSAFE result is replayed
against its concrete target pair.

The first naive implementation instead asked the exact SAT route to rediscover
the same relation. The fixed horizon-2 cohort exceeded the configured UNSAT
proof-step limit, while release-mode explicit search exceeded its node-step
limit. The structural route therefore closes a real governed capability that
the current exact fallback refuses. It is not a universal performance result:
the 460-byte structural admission is shared with other channel-family queries,
and the pair artifact does not yet have a canonical wire codec.

The separate two-channel complement control checks equality against inverted
difference patterns for four trace shapes. Every answer and earliest bad frame
agrees. The six-channel frame-zero rows also agree with direct exact checking,
and the structural cohort includes UNSAFE results at both frame zero and frame
one. A maintained Yosys plus Z3 temporal comparison remains open.
