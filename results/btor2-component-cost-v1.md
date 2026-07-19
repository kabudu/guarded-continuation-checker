# BTOR2 component-contract verification cost v1

This release-mode microbenchmark compares complete explicit layers, the
existing monolithic braking certificate, and the source-separated component
certificate. Each row records the median of 21 alternating process invocations.

| Case | Explicit bytes | Monolithic bytes | Component bytes | Explicit verify | Monolithic verify | Component verify | Component versus explicit | Component overhead versus monolithic |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Braking base | 1,180,313 | 386 | 494 | 38,129 us | 26 us | 35 us | 1,089.40x | 1.35x |
| Reused controller, fast plant | 287,786 | 385 | 493 | 9,122 us | 26 us | 36 us | 253.39x | 1.38x |
| Motor stop | 453,342 | 386 | 494 | 14,677 us | 26 us | 35 us | 419.34x | 1.35x |

Environment: Rust 1.97.0 release build on Darwin 25.5.0 arm64. The component
checker verifies two source models and a contract, which costs 35% to 38% more
than checking the equivalent monolithic specialised certificate. It remains
253x to 1,089x faster than explicit-layer verification. Component artifacts are
107 to 108 bytes larger than monolithic specialised artifacts.

These results reject a single-pair performance novelty claim. They retain the
modularity benefit and establish the baseline a future reusable component proof
must beat. Timing is not a CI golden or cross-platform guarantee.
