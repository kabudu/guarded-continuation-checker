# BTOR2 braking self-service acceptance v1

This retained run simulates the documented external-evaluator workflow. It is
not independent partner evidence.

The no-overwrite harness produces and separately verifies six answer-balanced
cases while checking the expected result, timing-free backend, stable selection
reason, and source SHA-256 digest. Both admitted SAFE cases select
`braking-phases`. Their unsafe boundaries and both semi-implicit near-neighbour
cases select exact search.

All six cases were accepted. The machine-readable evidence is retained in
[`btor2-braking-self-service-acceptance-v1.csv`](btor2-braking-self-service-acceptance-v1.csv).

Reproduce it from the repository root:

```sh
cargo build --locked
scripts/run-btor2-braking-self-service-acceptance.sh \
  target/debug/guarded-continuation-checker \
  /tmp/btor2-braking-self-service-acceptance-v1.csv
diff -u results/btor2-braking-self-service-acceptance-v1.csv \
  /tmp/btor2-braking-self-service-acceptance-v1.csv
```

This establishes deterministic self-service routing for bundled
product-shaped fixtures only. It does not validate continuous mechanics, an
unmodified public robot controller, deployment integration, or partner
suitability.
