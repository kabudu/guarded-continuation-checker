# BTOR2 braking-phase cohort v1

This answer-balanced cohort tests two admitted resettable braking controllers
and one deliberately rejected semi-implicit near-neighbour. Every row is
produced and independently verified through bounded portfolio v3.

| Model | Horizon | Answer | Backend | Explicit bytes | Portfolio bytes | Reduction |
|---|---:|---|---|---:|---:|---:|
| Braking controller | 255 | SAFE | braking-phases | 1,180,313 | 386 | 99.97% |
| Braking controller | 256 | UNSAFE | explicit-search | 473 | 473 | 0.00% |
| Motor emergency stop | 159 | SAFE | braking-phases | 453,342 | 386 | 99.91% |
| Motor emergency stop | 160 | UNSAFE | explicit-search | 377 | 377 | 0.00% |
| Semi-implicit braking | 127 | SAFE | explicit-search | 283,227 | 283,227 | 0.00% |
| Semi-implicit braking | 128 | UNSAFE | explicit-search | 345 | 345 | 0.00% |

The result supports exact certificate compression and timing-free routing for
the admitted source language. It does not establish general braking semantics,
continuous plant validity, novelty, or production readiness. The rejected
variant updates position from the new velocity and therefore cannot reuse the
certified relation.

Reproduce the deterministic table from the repository root:

```sh
cargo build --locked
scripts/run-btor2-braking-cohort.sh \
  target/debug/guarded-continuation-checker \
  /tmp/btor2-braking-cohort-v1.csv
diff -u results/btor2-braking-cohort-v1.csv \
  /tmp/btor2-braking-cohort-v1.csv
```
