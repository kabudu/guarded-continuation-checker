# Continuation Quotient SAT

Research implementation of exact continuation-quotient compilation for Boolean
satisfiability problems with small residual state spaces.

The method processes variables in a fixed order, canonicalizes the residual CNF
after each Boolean choice, and merges prefixes only when their residual formulas
are exactly identical. A representative path is retained so satisfiable terminal
states reconstruct complete assignments.

## Status

This is a research release, not a general-purpose replacement for CDCL SAT
solvers and not evidence that P = NP.

Validated findings:

- A structural frontier gate safely rejects formulas whose conservative residual
  vocabulary exceeds a fixed budget.
- On admitted, stable, repeated-query workloads, compiled assumption queries can
  outperform persistent Varisat while returning complete witnesses.
- Exact local insertion repair is supported.
- Deletions rebuild from the root. Canonical residuals do not retain the source
  multiplicity required for safe suffix-only deletion repair.
- Automatic selector policies did not generalize out of sample; CDCL remains the
  default for unknown workloads.
- All 40 bundled conventional SATLIB cases were rejected by the conservative
  gate. The technique therefore targets a narrow structured regime.

See [Research findings](docs/FINDINGS.md) and
[Reproducibility](docs/REPRODUCIBILITY.md) before interpreting benchmark results.

## Build and test

```sh
cargo test
cargo build --release
```

The executable is `target/release/continuation-quotient-sat`.

## Real DIMACS evaluation

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-dimacs \
  examples/modular-demo.cnf 10000 10 results/local-modular.csv
```

The command reports structural admission, compilation cost, state and artifact
sizes, repeated-query performance against persistent Varisat, agreement, and
witness validity.

## Repeated-query experiment

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-reuse \
  banded-planted 100 4 98302 20000 results/local-reuse.csv
```

## Repository layout

- `src/main.rs`: solver, generators, benchmarks, and regression tests.
- `examples`: small admitted demonstration formula.
- `results`: curated CSV summaries supporting the release claims.
- `docs`: findings, limitations, and exact reproduction commands.

The source preserves the broader experimental harness because negative results
and retractions are part of the reproducible record. Production extraction into
a smaller library is intentionally deferred until a real application corpus
demonstrates sufficient gate coverage.

Third-party SATLIB formulas are not redistributed because their licensing was
not established for this release. The derived aggregate summary is retained;
users may supply independently obtained DIMACS files to reproduce corpus scans.

## License

Apache License 2.0. See [LICENSE](LICENSE). The patent grant is appropriate for
an algorithmic research implementation intended for possible later reuse.

## Citation

See [CITATION.cff](CITATION.cff).
