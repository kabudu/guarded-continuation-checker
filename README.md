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
- On a temporal equality model, a repeated-transition kernel preserved the exact
  quotient and full witness recovery without replaying identical layers. It beat
  persistent Varisat at every admitted phase point; this is a deliberately narrow
  model-checking subclass, not a generic SAT result.
- A CNF-only recognizer now admits five fixed deterministic transition families:
  copy, negation, permutation, pairwise XOR, and a three-input Boolean circuit.
  It verifies that the normalized transition template repeats before constructing
  a logarithmic jump table; changed or unknown templates are rejected.
- A bounded exact-composition recognizer removes the named-rule requirement. It
  enumerates the repeated one-step CNF relation, admits it only when every current
  state has exactly one successor, and rejects incomplete, nondeterministic, or
  changing transitions.
- A local-cone recognizer removes that state-pair scan when every one-step clause
  constrains exactly one output. It recovers each output truth table independently
  and rejects cross-output clauses.
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

## Temporal phase experiment

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-temporal-phase \
  2,4,6 10,100,1000,10000 100 12 424242 results/local-temporal.csv
```

This controlled family holds a `width`-bit state constant across a `horizon` of
local CNF transitions. The benchmark reports both a dense quotient traversal and
an exact repeated-transition kernel against persistent Varisat.

## Recognized transition vocabulary

```sh
./target/release/continuation-quotient-sat \
  benchmark-temporal-vocabulary \
  copy,negate,permute,xor,circuit \
  4,6,8 10,100,1000 100 8 777 results/local-vocabulary.csv
```

The recognizer receives CNF rather than the generator's transition label. It
recovers local truth tables, matches only the fixed vocabulary, verifies exact
template repetition, and reconstructs complete witnesses for every admitted
query.

## Exact composed transitions

```sh
./target/release/continuation-quotient-sat \
  benchmark-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 10,100,1000 100 8 12345 results/local-compositions.csv
```

This path recognizes the complete deterministic one-step relation rather than
matching individual output functions to the fixed vocabulary. Its exhaustive
recognition cost is exponential in twice the state width, so width eight remains
an explicit hard gate.

For separable output cones, use the cheaper recognizer:

```sh
./target/release/continuation-quotient-sat \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,8,12 10,100,1000 100 12 24680 results/local-cones.csv
```

Semantic recovery costs roughly the sum of the local truth-table sizes rather
than `2^(2*width)`. The explicit jump kernel still contains `2^width` states, so
this removes one exponential factor but does not make unbounded-width SAT easy.

To avoid that explicit state table for deterministic trajectory queries:

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  16,32 10,100,500 50 32 4242001 results/symbolic-replay.csv
```

This representation stores only the recovered local functions and replays them
directly. It scales linearly with width for bounded local dependency, but requires
a fully specified initial state and takes linear time in the horizon.

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
