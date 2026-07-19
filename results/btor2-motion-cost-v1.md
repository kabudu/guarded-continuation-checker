# BTOR2 coupled-motion verification cost v1

This release-mode microbenchmark compares independent verification of complete
explicit SAFE layers with verification of the exact coupled-motion certificate.
Each row records the median of 21 alternating process invocations after both
artifacts were produced.

| Model | Horizon | Explicit bytes | Motion bytes | Explicit verify median | Motion verify median | Speedup |
|---|---:|---:|---:|---:|---:|---:|
| Motion envelope | 200 | 624,272 | 358 | 17,726 us | 22 us | 805.73x |
| Servo motion | 128 | 253,928 | 358 | 7,003 us | 21 us | 333.48x |

Environment: Rust 1.97.0 release build on Darwin 25.5.0 arm64. The measurements
include certificate semantic verification inside the CLI process, but exclude
source and certificate file opening and certificate decoding. Process startup
is outside the reported internal timer.

This is a small local microbenchmark, not a cross-platform latency guarantee.
The artifact size result is deterministic; wall-clock measurements are not CI
golden files. Reproduce a fresh measurement with:

```sh
cargo build --release --locked
scripts/run-btor2-motion-cost.sh \
  target/release/guarded-continuation-checker \
  /tmp/btor2-motion-cost-v1.csv
```

The checker uses logarithmic affine matrix powering while the producer uses a
different triangular closed form. The speedup therefore reflects both the
smaller proof and avoidance of explicit successor reconstruction.
