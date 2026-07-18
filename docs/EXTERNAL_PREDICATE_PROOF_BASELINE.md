# External predicate-proof baseline v1

This experiment replaces the native Varisat proof stream inside predicate
certificate v2 with a maintained external SAT proof producer and an independent
checker. It compares the same relation- and terminal-completeness obligations;
it is not a comparison with a differently scoped model-checking property.

The result closes one part of the external-tool comparison gate. It does not
establish scholarly novelty or production readiness.

## Baseline selection

The pinned tools are:

- [CaDiCaL 3.0.0](https://github.com/arminbiere/cadical/tree/7b99c07f0bcab5824a5a3ce62c7066554017f641),
  commit `7b99c07f0bcab5824a5a3ce62c7066554017f641`, as proof producer; and
- [DRAT-trim v05.22.2023](https://github.com/marijnheule/drat-trim/tree/2e5e29cb0019d5cfd547d4208dca1b3ec290349f),
  commit `2e5e29cb0019d5cfd547d4208dca1b3ec290349f`, as independent checker.

CaDiCaL accepts DIMACS and emits a DRAT proof; DRAT-trim checks that the proof
establishes UNSAT. CaDiCaL's text DRAT mode is selected explicitly because the
pinned DRAT-trim release rejected one valid CaDiCaL 3.0 binary stream while the
semantically identical text stream verified. The harness treats any solver
status other than 20, checker status other than zero, or missing `s VERIFIED`
line as failure.

[Certifaiger](https://fmv.jku.at/certifaiger/) remains the closest maintained
hardware-certificate system. It was used to check bit-level safety certificates
in HWMCC 2024 and 2025. Its witness-circuit obligation proves unbounded hardware
safety, whereas this experiment proves the bounded predicate relation and
terminal-set completeness claims carried by GCC's counterfactual workflow.
Calling those obligations equivalent would be inaccurate, so a direct
Certifaiger timing row is not substituted for this baseline.

## Deterministic export

`export-aiger-predicate-v2-obligations` reads a source-bound canonical v2
certificate and publishes a no-clobber directory containing:

- one canonical DIMACS file for every phase/source relation-completeness claim;
- one canonical terminal-completeness DIMACS file;
- `aggregate.cnf`; and
- a canonical manifest binding the model, certificate, dimensions, filenames,
  kinds and SHA-256 digests.

The aggregate is not a lossy shortcut. If obligation `i` has formula `F_i`, its
variables are placed in a private block and every clause is guarded by selector
`s_i`. A final clause requires at least one selector:

```text
(not s_i or C) for every clause C in F_i
(s_0 or s_1 or ... or s_n)
```

The aggregate is satisfiable exactly when at least one original obligation is
satisfiable. Therefore one checked aggregate UNSAT proof establishes that every
exported completeness obligation is UNSAT. Individual files remain present and
are also checked by the experiment, so the aggregation cannot hide which
obligations were represented.

## Preserved result

All 40 individual obligations and all four aggregates verified on Ubuntu 24.04
under a 300-second per-process deadline, 2-GiB address-space ceiling and
512-MiB proof-file ceiling.

| Cohort | Result | Individual obligations | Aggregate CNF | Aggregate DRAT | Produce | Check | Total |
|---|---:|---:|---:|---:|---:|---:|---:|
| interrupt, 9 inputs, horizon 8 | avoidable | 5 | 12,549 B | 14,169 B | 6.786 ms | 56.851 ms | 63.637 ms |
| actuator, 12 inputs, horizon 1 | unavoidable | 9 | 15,980 B | 18,427 B | 6.639 ms | 57.671 ms | 64.309 ms |
| actuator, 12 inputs, horizon 16 | avoidable | 9 | 16,381 B | 18,211 B | 4.104 ms | 33.702 ms | 37.806 ms |
| sensor, 16 inputs, horizon 32 | avoidable | 17 | 42,883 B | 52,244 B | 11.461 ms | 57.149 ms | 68.610 ms |

The corresponding native-v2 checker averages 0.311--0.882 ms on these cohorts,
so the external checker is not a performance win. Its value is format- and
implementation-diverse validation. External proof production takes 4.104--11.461
ms for the completeness layer alone; native full-certificate production
averages 37.379--43.128 ms but also builds relations, witnesses, powers and the
final trace. Those times are not interchangeable end-to-end measurements.

Raw results:

- [`external-predicate-proof-interrupt-h8-v1.csv`](../results/external-predicate-proof-interrupt-h8-v1.csv)
- [`external-predicate-proof-actuator-h1-unavoidable-v1.csv`](../results/external-predicate-proof-actuator-h1-unavoidable-v1.csv)
- [`external-predicate-proof-actuator-h16-v1.csv`](../results/external-predicate-proof-actuator-h16-v1.csv)
- [`external-predicate-proof-sensor-h32-v1.csv`](../results/external-predicate-proof-sensor-h32-v1.csv)

## Reproduction

Build the project, produce a v2 certificate, and export its obligations:

```bash
cargo build --release
target/release/guarded-continuation-checker \
  export-aiger-predicate-v2-obligations \
  MODEL.aag CERTIFICATE.cert2 target/obligations
```

On Ubuntu 24.04 with Git, a C/C++ toolchain and GNU coreutils:

```bash
scripts/build-external-proof-baseline-tools.sh target/external-proof-tools
scripts/run-external-predicate-proof-baseline.sh \
  target/obligations \
  target/external-proof-tools/bin/cadical \
  target/external-proof-tools/bin/drat-trim \
  target/external-proof-results.csv
```

The build script refuses to replace its destination and checks out immutable
commits. The run script validates every digest, refuses linked CNFs and existing
output, checks every individual proof, checks the aggregate proof, and records
tool-binary digests in every result row.

## Remaining gap

This establishes an obligation-equivalent maintained external SAT-proof
baseline and a deterministic translation boundary. It does not compare GCC's
complete certificate trust base with Certifaiger's complete witness-circuit
trust base, does not provide a formally verified checker, and does not include
external expert review. Those remain open novelty gates.
