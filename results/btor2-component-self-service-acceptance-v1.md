# BTOR2 component self-service acceptance v1

This retained run simulates a self-service evaluator using separate controller,
plant, and contract files. It is not independent partner evidence.

All eight answer-balanced cases were accepted. The harness checks each source
digest, expected answer, timing-free backend, selection reason, and separate
verification result. It includes reuse of one unchanged controller with two
plants and both answers for a rejected semi-implicit plant.

Reproduce the machine-readable evidence from the repository root:

```sh
cargo build --locked
scripts/run-btor2-component-self-service-acceptance.sh \
  target/debug/guarded-continuation-checker \
  /tmp/btor2-component-self-service-acceptance-v1.csv
diff -u results/btor2-component-self-service-acceptance-v1.csv \
  /tmp/btor2-component-self-service-acceptance-v1.csv
```

This proves the bundled workflow is deterministic and does not require
formula-specific calibration. It does not establish partner suitability,
unmodified public-product validity, continuous mechanics, or production
readiness.
