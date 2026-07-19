# BTOR2 bounded-search cohort v1

This six-query run exercises both answers at the exact safety boundary for
three word-level firmware models. Every generated certificate was immediately
decoded and verified against its source.

| Model | SAFE bound | SAFE bytes | UNSAFE bound | UNSAFE bytes |
|---|---:|---:|---:|---:|
| Watchdog | 2 | 382 | 3 | 216 |
| Actuator position | 200 | 505,396 | 201 | 418 |
| Saturating timer | 254 | 802,525 | 255 | 472 |

The exact results are retained in
[`btor2-search-cohort-v1.csv`](btor2-search-cohort-v1.csv). Reproduce them with:

```sh
cargo build --release --locked
scripts/run-btor2-search-cohort.sh \
  target/release/guarded-continuation-checker /tmp/btor2-search.csv
```

The evidence is deliberately negative as well as positive. UNSAFE witnesses
remain a few hundred bytes, while complete SAFE reachable-layer certificates
grow to hundreds of kilobytes on tiny one-word models. This makes explicit
state enumeration a useful exact fallback and oracle, but not the desired
breakthrough. The next composition experiment must reduce SAFE evidence growth
while proving the same complete-successor obligation.
