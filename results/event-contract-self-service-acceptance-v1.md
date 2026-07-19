# Event-contract self-service acceptance v1

This retained acceptance result simulates the workflow an external evaluator
can run without assistance. It is not independent partner evidence and must not
be presented as such.

The harness ran the release binary over six product-shaped cases, split evenly
between avoidable and unavoidable results. For every case it:

1. invoked the timing-free event-contract portfolio;
2. independently replayed the resulting report against the original model,
   selected bad output, contract, and certificate;
3. checked the expected answer; and
4. retained the source and contract SHA-256 digests.

All six cases were accepted. The machine-readable evidence is
[`event-contract-self-service-acceptance-v1.csv`](event-contract-self-service-acceptance-v1.csv).

Reproduce it from the repository root:

```sh
cargo build --release
scripts/run-event-contract-self-service-acceptance.sh \
  target/release/guarded-continuation-checker \
  /tmp/event-contract-self-service-acceptance-v1.csv
```

The script refuses to overwrite existing output and supports both Linux
`sha256sum` and macOS `shasum`.
