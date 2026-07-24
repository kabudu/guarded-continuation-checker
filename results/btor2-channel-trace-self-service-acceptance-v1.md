# BTOR2 channel trace self-service acceptance v1

This retained run simulates an evaluator following the repository's documented
file workflow with a built GCC executable and clean copies of the published
model, query manifest, and resource policy. It is not independent partner
evidence.

The six predeclared cases cover capability discovery, complete 42-query
production, fresh-process verification, create-new collision preservation,
query-binding rejection, and typed resource refusal with no result artifact.
The accepted production artifact is 4,899,434 bytes with SHA-256
`9ca8d6bdb0ee10877a29711fbb518810b28908b64b831283db5e2db3688ecf4a`.
No case changes a formula-specific threshold or solver route.

Reproduce the machine-readable evidence from the repository root:

```sh
cargo build --release --locked
scripts/run-btor2-channel-trace-self-service-acceptance-v1.sh \
  target/release/guarded-continuation-checker \
  /tmp/btor2-channel-trace-self-service-acceptance-v1.csv
diff -u results/btor2-channel-trace-self-service-acceptance-v1.csv \
  /tmp/btor2-channel-trace-self-service-acceptance-v1.csv
```

This proves that the bundled workflow is deterministic and usable without
per-formula calibration. It does not establish partner suitability, production
readiness, or novelty.
