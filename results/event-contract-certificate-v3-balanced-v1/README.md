# Event-contract certificate v3 answer-balanced cohort

This cohort freezes three product-shaped avoidable contracts and three
product-shaped unavoidable contracts after the structural admission rule was
defined. The unavoidable contracts are satisfiable fixed-event sequences that
drive each model's declared bad output, not contradictory assumptions.

Each CSV contains ten release-mode trials. All 60 rows agree with separately
encoded exact CDCL and pass independent certificate verification.

| Contract | Result | Generation | Verification | Exact CDCL | V3/CDCL | Artifact | Proofs |
|---|---:|---:|---:|---:|---:|---:|---:|
| Interrupt priority | avoidable | 10.705 ms | 0.644 ms | 0.253 ms | 2.55x | 17,674 B | 9 |
| Actuator interlock | avoidable | 18.147 ms | 0.743 ms | 0.317 ms | 2.34x | 27,838 B | 17 |
| Robot recovery | avoidable | 66.515 ms | 1.570 ms | 0.579 ms | 2.71x | 84,033 B | 33 |
| Interrupt hazard | unavoidable | 9.792 ms | 0.474 ms | 0.058 ms | 8.23x | 9,494 B | 5 |
| Actuator hazard | unavoidable | 9.985 ms | 0.456 ms | 0.052 ms | 8.79x | 11,329 B | 9 |
| Robot hazard | unavoidable | 10.214 ms | 0.687 ms | 0.054 ms | 12.73x | 26,610 B | 17 |

Values are medians of ten trials measured on 19 July 2026 with Rust 1.97.0 on
Apple Silicon. V3 verification is slower than solving every individual row.
The portfolio therefore admits v3 for deterministic evidence, not for a speed
claim, and uses exact CDCL whenever the certificate regime is structurally
inapplicable or encounters a recognized bounded resource failure.

Reproduce into a fresh directory:

```sh
cargo build --release
scripts/benchmark-event-contract-v3-balanced.sh /tmp/event-v3-balanced 10
```
