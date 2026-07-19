# BTOR2 exact word-region cohort v1

This retained experiment compares the new static bounded portfolio with the
explicit exact-search certificate on the same six boundary queries.

| Model | Horizon | Answer | Selected backend | Explicit bytes | Portfolio bytes | Reduction |
|---|---:|---|---|---:|---:|---:|
| Watchdog | 2 | SAFE | word-region | 382 | 298 | 21.99% |
| Watchdog | 3 | UNSAFE | explicit-search | 216 | 216 | 0.00% |
| Actuator | 200 | SAFE | word-region | 505,396 | 304 | 99.94% |
| Actuator | 201 | UNSAFE | explicit-search | 418 | 418 | 0.00% |
| Saturating timer | 254 | SAFE | word-region | 802,525 | 312 | 99.96% |
| Saturating timer | 255 | UNSAFE | explicit-search | 472 | 472 | 0.00% |

Every row is independently decoded and verified from its original BTOR2
source. The portfolio rule uses no timings, training, or formula calibration.
It selects a word-region proof only when the source recogniser proves the exact
recurrence and bad-predicate language. Every other query is passed unchanged to
the explicit exact backend.

The two large SAFE reductions are the positive result. The watchdog reduction
is small because its explicit certificate has only six logical states. The
UNSAFE rows intentionally do not improve because their concrete witnesses were
already compact.

This is a narrow synthetic and product-shaped cohort, not broad product
validity. The command below reproduces the CSV without overwriting an existing
result:

```sh
cargo build --release
scripts/run-btor2-region-cohort.sh \
  target/release/guarded-continuation-checker /tmp/btor2-region-cohort-v1.csv
diff -u results/btor2-region-cohort-v1.csv \
  /tmp/btor2-region-cohort-v1.csv
```
