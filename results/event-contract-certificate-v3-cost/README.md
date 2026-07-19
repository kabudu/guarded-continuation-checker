# Event-contract certificate v3 cost

These CSV files preserve ten release-mode trials for the canonical v3 producer,
independent verifier, and separately encoded exact CDCL control. Every row binds
the source AIG and named event contract by SHA-256, records exact answer
agreement, and includes complete generation, verification, artifact, proof, and
direct-evaluation costs.

The evidence was generated on 19 July 2026 with Rust 1.97.0 on an Apple Silicon
Mac. Timings are evidence for this machine and cohort, not portable guarantees.

Reproduce into a fresh directory:

```sh
cargo build --release
./scripts/benchmark-event-contract-certificate-v3.sh /tmp/gcc-event-v3-cost 10
```

The three product-shaped rows are avoidable. The additional actuator row binds
a satisfiable fixed-input event contract that makes avoidance impossible, so
both answer classes are represented. Verification is deliberately reported
against CDCL even though independent evidence checking performs more work and
is slower on every row.
