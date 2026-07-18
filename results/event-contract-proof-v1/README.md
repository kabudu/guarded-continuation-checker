# Event-contract proof primitive v1

These CSV files preserve ten release-mode trials for independently checked
CNF-constrained relation and terminal-set proof primitives. The source AIG and
named contract SHA-256 digests bind every row.

The evidence was generated on 19 July 2026 with Rust 1.97.0 on an Apple Silicon
Mac. Timings are evidence for this machine and cohort, not portable guarantees.

Reproduce into a fresh directory:

```sh
cargo build --release
./scripts/benchmark-event-contract-proofs.sh /tmp/gcc-event-proofs 10
```

This experiment does not emit certificate v3 and does not check powered phase
rows or a final answer. It establishes that the required one-step and terminal
proof obligations can be generated and checked before the artifact contract is
frozen.
