# Event-contract CNF experiment v1

These CSV files preserve ten release-mode trials per product-shaped fixture.
Every row records exact agreement with separately encoded persistent CDCL and a
successful replay of any reconstructed witness against the source AIG.

The evidence was generated on 19 July 2026 with Rust 1.97.0 on an Apple Silicon
Mac. Timing is evidence for this machine and cohort, not a portable guarantee.
The raw source and contract SHA-256 bindings are included in every row.

Reproduce into a fresh directory:

```sh
cargo build --release
./scripts/benchmark-event-contracts.sh /tmp/gcc-event-contracts 10
```

The retained result is negative for performance. Median predicate/CDCL query
ratios are 1.09x, 11.03x, and 36.20x slower for the interrupt, actuator, and
robot fixtures respectively. The experiment establishes exact CNF contract
semantics, not portfolio admission or a speed claim.
