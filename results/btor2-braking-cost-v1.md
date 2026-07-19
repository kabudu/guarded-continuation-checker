# BTOR2 braking-phase verification cost v1

This release-mode microbenchmark compares independent verification of complete
explicit SAFE layers with verification of the exact phase-composition
certificate. Each row records the median of 21 alternating process invocations
after both artifacts were produced.

| Model | Horizon | Explicit bytes | Braking bytes | Explicit verify median | Braking verify median | Speedup |
|---|---:|---:|---:|---:|---:|---:|
| Braking controller | 255 | 1,180,313 | 386 | 38,958 us | 25 us | 1,558.32x |
| Motor emergency stop | 159 | 453,342 | 386 | 15,250 us | 25 us | 610.00x |

Environment: Rust 1.97.0 release build on Darwin 25.5.0 arm64. Measurements
include semantic verification inside the CLI process but exclude source and
certificate file opening and certificate decoding. Process startup is outside
the reported internal timer.

This is a small local microbenchmark, not a cross-platform latency guarantee.
Artifact size is deterministic; wall-clock measurements are not CI golden
files. Reproduce a fresh measurement with:

```sh
cargo build --release --locked
scripts/run-btor2-braking-cost.sh \
  target/release/guarded-continuation-checker \
  /tmp/btor2-braking-cost-v1.csv
```

The producer uses direct polynomial phase formulas. The checker uses boundary
inequalities and independently arranged first/last arithmetic-series sums.
The speedup therefore reflects the smaller proof and avoidance of explicit
successor reconstruction.
