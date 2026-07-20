# Proof-carrying controller MTBDD whole-process baseline v1

## Question

Does the proof profile retain its in-process verification improvement when a
self-service evaluator pays model loading, artifact decoding, manifest checks,
proof checking, plant replay, output formatting, and process startup?

## Method

`scripts/benchmark-controller-proof-mtbdd-process.sh` runs the compact and
proof-carrying create and verify commands on the same public six-property
physical-plant manifest. Each trial creates fresh artifacts. It compares every
ordered member line before accepting a row, permits 1 to 20 predeclared trials,
and refuses to overwrite output. The retained arm64 release-build run used five
trials.

## Result

| Operation | Compact median | Proof median | Proof improvement |
| --- | ---: | ---: | ---: |
| Create and self-check | 2.390 s | 1.454 s | 1.64x |
| Fresh verification | 0.964 s | 0.483 s | 2.00x |

All 20 rows preserve exact ordered answer and trace agreement. Compact checks
131,072 assignments and writes 8,549 bytes. Proof checks the UNSAT miter without
assignment replay and writes 251,221 bytes, 29.39 times larger.

This is a positive local whole-process result, not a universal speed claim.
The complementary three-trial arm64 resource run records a negative memory
tradeoff: median proof peak RSS is 21.22 MiB versus 16.41 MiB for creation, and
9.61 MiB versus 4.02 MiB for verification. The speed profile therefore buys
faster checking with larger transferred evidence and higher checker memory.
Hosted replication and identical-scope comparison with the maintained formal
workflow remain required. The static compact portfolio stays unchanged; no
timing or memory observation participates in routing.
