# OpenTitan PWM trace-predicate portfolio v1

Status: exact work reduction passes; end-to-end benefit is falsified

## Question

After independently admitting bounded reachable channel classes once, can GCC
evaluate masked temporal predicates on one representative per class, retain
direct exact evaluation for singleton classes, and beat direct evaluation for
a complete authentic PWM batch?

## Static contract

Each query binds an ordered query identifier, channel, inclusive frame window,
mask and masked value. Query identifiers must be strictly increasing. Counts,
channels, windows, masks and values are validated before evaluation. The
portfolio accepts only an opaque admission capability created by independent
source replay of the canonical equivalence artifact. Invalid evidence never
falls back. Singleton classes use the direct exact route; only non-singleton
classes reuse a representative result.

The exact baseline parses and replays the same source and evaluates every
logical query independently. Both paths return match status and the earliest
matching frame. The retained test uses legitimate one-bit observation
predicates over different suffix windows through frame 4,095. It does not use
out-of-width impossible values.

## Retained result

The complete four-row, five-trial result is
[`results/opentitan-pwm-trace-predicate-portfolio-arm64-v1.csv`](../results/opentitan-pwm-trace-predicate-portfolio-arm64-v1.csv).
No row was selected or discarded.

| Predicates per channel | Logical queries | Candidate evaluations | Direct evaluations | Work reduction | Speedup |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 16 | 96 | 64 | 96 | 33.333333% | 1.005137× |
| 256 | 1,536 | 1,024 | 1,536 | 33.333333% | 1.002106× |
| 4,096 | 24,576 | 16,384 | 24,576 | 33.333333% | 0.999184× |
| 8,192 | 49,152 | 32,768 | 49,152 | 33.333333% | 0.996181× |

Every candidate answer and earliest frame equals the direct exact result. The
portfolio always removes one evaluation for each repeated two-channel class,
but it produces no stable end-to-end runtime advantage. Parsing, exact
admission and trace handling dominate this model; cache lookup consumes the
small remaining saving.

## Decision

The opaque admission and exact routing semantics are useful production
mechanisms, but simple bounded trace scans are too cheap to justify this reuse
layer on the current source. The result does not pass a product-performance or
novelty gate. Further cache tuning is not the next experiment.

The next candidate must reuse an expensive independently checked property
obligation, not just a scan over already materialised trace values. It must
retain a complete result artifact, compare proof production and verification
against maintained tools at identical scope, and show an advantage after the
one-time equivalence admission cost.
