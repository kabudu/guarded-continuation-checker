# External event-contract proof baseline v3

These files retain one complete cross-check of every relation and terminal
completeness obligation carried by the four event-contract certificate v3
fixtures. CaDiCaL 3.0.0 produced canonical text DRAT and DRAT-trim
v05.22.2023 checked every individual proof and the selector aggregate.

| Fixture | Result | Obligations | Aggregate CNF | Aggregate DRAT | Produce | Check | Total |
|---|---:|---:|---:|---:|---:|---:|---:|
| Interrupt priority | avoidable | 9 | 23,604 B | 26,864 B | 9.620 ms | 31.556 ms | 41.176 ms |
| Actuator interlock | avoidable | 17 | 32,542 B | 37,223 B | 9.532 ms | 30.069 ms | 39.601 ms |
| Robot recovery | avoidable | 33 | 91,223 B | 111,411 B | 12.670 ms | 32.949 ms | 45.619 ms |
| Actuator fixed-input | unavoidable | 9 | 15,980 B | 18,427 B | 9.017 ms | 30.443 ms | 39.460 ms |

All 68 individual obligations and all four aggregates verified. Each CSV row
binds the exported manifest and both external tool binaries by SHA-256. The
measurements were taken on 19 July 2026 on Apple Silicon. They demonstrate
proof-format and checker diversity, not portable performance.

Reproduce from a release build:

```sh
scripts/build-external-proof-baseline-tools.sh target/event-v3-tools
scripts/run-external-event-contract-proof-baseline.sh \
  target/event-v3-tools /tmp/event-v3-external \
  target/release/guarded-continuation-checker
```
