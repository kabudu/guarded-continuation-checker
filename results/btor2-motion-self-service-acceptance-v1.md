# BTOR2 motion self-service acceptance v1

This retained run simulates the workflow available to an external evaluator.
It is not independent partner evidence and must not be described as such.

The no-overwrite harness uses only the documented bounded-portfolio commands.
For two motion boundaries and one rejected near-neighbour boundary, it:

1. produces a certificate from the original source, property, and horizon;
2. checks the expected answer, timing-free backend, and stable selection reason;
3. invokes the separate verification command against the original source; and
4. retains the source SHA-256 digest and acceptance result.

All six answer-balanced cases were accepted. Both admitted SAFE cases selected
`motion-curve`. Their UNSAFE boundaries and both near-neighbour answers selected
`explicit-search`. The machine-readable evidence is retained in
[`btor2-motion-self-service-acceptance-v1.csv`](btor2-motion-self-service-acceptance-v1.csv).

Reproduce it from the repository root:

```sh
cargo build
scripts/run-btor2-motion-self-service-acceptance.sh \
  target/debug/guarded-continuation-checker \
  /tmp/btor2-motion-self-service-acceptance-v1.csv
diff -u results/btor2-motion-self-service-acceptance-v1.csv \
  /tmp/btor2-motion-self-service-acceptance-v1.csv
```

This validates usability and deterministic routing for the bundled
product-shaped fixtures. It does not validate an unmodified public robot,
continuous mechanics, deployment integration, or partner suitability.
