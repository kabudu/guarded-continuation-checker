# Continuation Quotient SAT

Research implementation of exact continuation-quotient compilation for Boolean
satisfiability problems with small residual state spaces.

The current product-facing backend is **CQ-SAT/GCC**: Continuation-Quotient SAT
with Global Checkpoint Clauses, wrapped in a calibration-free portfolio gate that
uses persistent CDCL outside its validated structural regime.

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
The enforced gaps that still prevent a production claim are tracked in
[Production-readiness gates](docs/PRODUCTION_READINESS.md).
Operators evaluating the tool with a design partner must follow the
[production-evaluation operations runbook](docs/OPERATIONS.md).
Untrusted inputs must use the probed, fail-closed
[hostile-RTL isolation profile v1](docs/ISOLATION_PROFILE_V1.md).
Any use in a regulated or safety-related programme must follow the bounded
[standards applicability and assurance claims](docs/STANDARDS_APPLICABILITY.md).
The remaining independent assessment and design-partner gates have fixed
[external evidence and pilot acceptance criteria](docs/EXTERNAL_EVIDENCE_PROTOCOL.md).
External engagement can start from the sendable
[design-partner brief](docs/DESIGN_PARTNER_BRIEF.md), private
[pilot intake template](docs/PILOT_INTAKE_TEMPLATE.md), and
[independent-assessment statement of work](docs/INDEPENDENT_ASSESSMENT_SOW.md).
Partners operate the evaluation independently using repository resources and
return only the final non-confidential
[outcome and suitability report](docs/OUTCOME_REPORT_TEMPLATE.md).

An isolated research extension explores
[certified causal counterexample analysis](docs/CAUSAL_ANALYSIS.md). It computes
a replay-checked, 1-minimal sufficient set of input segments for an earliest
AIGER failure and compares continuation-quotient intervention reuse with fresh
and persistent CDCL. Results are published as an atomic, SHA-256-bound
[causal evidence bundle v1](docs/CAUSAL_BUNDLE_V1.md). This is a precisely
bounded experiment, not a claim that counterexample minimisation or causal
explanation is new.

The [closest-method comparison](docs/CAUSAL_STRATEGY_COMPARISON.md) replays
deletion and QuickXplain intervention transcripts through fresh CDCL,
persistent CDCL, and admitted CQ. It records the negative result that CQ does
not amortise its preparation cost on the current cohort.

The follow-on [compile-once causal batch](docs/CAUSAL_BATCH.md) shares one
maximum-horizon CNF and, when structurally admitted, one continuation quotient
across every reachable `(frame, bad-output)` target. It enumerates a bounded set
of distinct 1-minimal causes over segment, point, and dyadic observation
vocabularies and measures actual and projected break-even against persistent
CDCL. A separate command replays every published cause from the source model.

The first [Counterfactual Interface Quotient](docs/COUNTERFACTUAL_INTERFACE_QUOTIENT.md)
experiment stops expanding long constant input phases into individual frames.
It composes exact powered relations over the controller's latch-state boundary
and reconstructs a concrete trace only when requested. On the bounded
infusion-pump regression, ten trials per horizon produced median end-to-end
speedups of 1.42x, 2.38x, 4.15x, and 7.00x at horizons 8, 16, 32, and 64,
respectively, with exact CDCL agreement and validated witnesses. This is a
robust positive result on one narrow controller class, not a general or novel
algorithm claim. The broader sequence is tracked in the
[firmware and robotics research roadmap](docs/FIRMWARE_ROBOTICS_RESEARCH_ROADMAP.md).

The first public RTL compatibility corpus is under
[`corpus/rtl/yosys-simple`](corpus/rtl/yosys-simple/README.md). It pins five
unmodified upstream Yosys sources and exercises twelve separately authored
SAFE/UNSAFE properties across two Yosys versions. This is reproducible public
evidence, not a substitute for confidential design-partner validation.

## CQ-SAT/GCC portfolio

```sh
./target/release/continuation-quotient-sat \
  benchmark-cq-portfolio watchdog4 9 137,1333,7777 50 10 200000 4141414 \
  results/local-watchdog-portfolio.csv
```

The static gate uses only transition density, dependency fan-out, width, declared
query-batch size, and assumption density. It never trial-solves candidate
backends. Dense models up to width nine with at least eight queries and narrow hub
models up to width seven with at least 128 queries use CQ-SAT/GCC when the batch
averages no more than one state-width of assumptions; everything else uses the exact
persistent-CDCL path. The CSV records
the backend, reason, recognition cost, structural metrics, speedups, agreement,
and witness validity.

See the executable [watchdog/interlock](examples/watchdog-controller.md) and
[redundant sensor-voting](examples/redundant-sensor-monitor.md) examples for the
accelerated and safe-fallback paths.

### Product integration: firmware safety gate

The [infusion-pump firmware example](examples/products/infusion-pump/README.md)
shows the verifier as a pull-request safety gate starting from SystemVerilog. It
runs Yosys in an isolated staging directory, preserves signal names, and feeds
the generated ASCII AIGER model into the exact portfolio. A protected controller
passes; a realistic door-interlock regression rejects the build and produces the
shortest named input/state trace needed to reproduce the failure.

```sh
./target/release/continuation-quotient-sat \
  firmware-rtl-safety-gate \
  examples/products/infusion-pump/rtl/safe-controller.sv \
  infusion_pump_controller 100 target/firmware-safety
```

The command requires Yosys on `PATH`. It writes the source snapshot, synthesis
script and log, generated model and signal map, stable report and metrics,
provenance manifest, and GitHub Actions annotations. Exit statuses distinguish
safe builds (0), discovered violations (1), and tool or input failures (2). The
example includes a copyable workflow and independent SymbiYosys/Z3 oracle files.
It demonstrates integration mechanics, not medical-device certification.

Projects split across source files use the bounded project interface. Source
paths are copied to fixed staging names and never interpolated into Yosys code:

```sh
./target/release/continuation-quotient-sat \
  firmware-rtl-project-safety-gate infusion_pump_system 100 \
  target/firmware-safety/project \
  examples/products/infusion-pump/rtl/project/pump-components.sv \
  examples/products/infusion-pump/rtl/project/pump-system.sv
```

The project interface accepts at most 64 regular files, 10 MiB per file and
25 MiB total. It rejects canonical duplicates and publishes deterministic source
snapshots plus their ordered labels in the final manifest.

Representative projects use the strict config interface for includes,
parameters, clock/reset policy, and inferred memories:

```sh
./target/release/continuation-quotient-sat \
  firmware-rtl-config-safety-gate \
  examples/products/infusion-pump/rtl/config-project/cq-project.conf \
  target/firmware-safety/config-project
```

All paths are relative to the config and may not traverse its directory.
Sources, headers, and the config itself are bounded and snapshotted before
Yosys starts. Project config v1 supports a permanently deasserted reset. Config
v2 additionally supports `reset=SIGNAL:active-low:N` or `active-high:N`: reset
is asserted for exactly frames `0..N-1` and deasserted thereafter. Each frame
represents one declared active clock edge.

Constant environment contracts use a bounded assumptions file containing one
`NAME=0` or `NAME=1` entry per synthesized primary input. Each entry is enforced
at every frame and an unknown or duplicate name fails the run:

```sh
./target/release/continuation-quotient-sat \
  firmware-rtl-constrained-project-safety-gate \
  infusion_pump_controller 8 target/firmware-safety/door-closed \
  examples/products/infusion-pump/rtl/door-closed.assumptions \
  examples/products/infusion-pump/rtl/door-interlock-regression.sv
```

The assumptions file is copied into the evidence bundle and each resolved
constraint is recorded in the safety report. These assumptions describe the
verified environment; they are not proof that a deployed environment satisfies
that contract.

RTL synthesis is contained in a dedicated Unix process group. A timeout kills
the entire group, including descendants, and every run has a 512 MiB output-file
limit. Linux—the supported production-evaluation host—also enforces a 2 GiB
address-space limit. macOS remains supported for development and records
`synthesis_memory_limit_kind=unavailable`; its process-tree, file-size, timeout,
and model limits still apply, but it must not be used as evidence of hard memory
containment.

Completed evidence bundles use the strict, SHA-256-bound
[RTL artifact schema v4](docs/ARTIFACT_SCHEMA_V4.md). Validate one before
retention or downstream processing:

```sh
./target/release/continuation-quotient-sat \
  firmware-artifact-validate target/firmware-safety/project
```

Schema v4 additionally rejects modified or symlinked indexed evidence. SHA-256
detects changes relative to a trusted manifest; it is not a signature. See the
[security policy and threat model](SECURITY.md) before evaluating untrusted RTL.
Direct AIGER inputs are capped at 256 MiB. CI also runs 25,000 deterministic
mutations over persistent AIGER, assumptions, project-config, and CLI corpora.

The product-facing commands follow
[firmware CLI contract v2](docs/FIRMWARE_CLI_V2.md). Query both active contract
versions with `firmware-cli-version`; breaking command, argument, or exit-status
changes require a new CLI contract version.

The same example now includes a five-module controller and a repeated-property
BMC experiment:

```sh
cd examples/products/infusion-pump/rtl
yosys -Q -q -s synthesize-multimodule.ys
cd ../../../..
./target/release/continuation-quotient-sat \
  benchmark-aiger-query-reuse \
  examples/products/infusion-pump/rtl/multimodule-controller.aag \
  8,16,32,64 10 results/local-rtl-query-reuse.csv
```

The benchmark compares bounded two-property solver reuse with a fresh exact BMC
solver per property. A static gate permits reuse only for multi-property
encodings of at most 15,000 clauses; larger and single-property jobs use cold
BMC. It reports both strategies even when reuse loses.

### Standard AIGER safety verification

The portfolio ingests original five-field ASCII (`aag`) and binary (`aig`)
AIGER safety models directly. Closed models inside
the bounded deterministic regime remain eligible for CQ-SAT/GCC; primary-input
or wider models are sent directly to an exact Tseitin-unrolled CDCL backend.

```sh
./target/release/continuation-quotient-sat \
  verify-cq-aiger examples/aiger/counter-overflow-4.aag \
  137 10 200000 results/local-aiger-counter.csv \
  results/local-aiger-counter-safety.txt
```

The bundled [four-bit counter overflow model](examples/aiger/README.md) is an
independently authored, MIT-licensed model pinned to its upstream revision. Its
bad-state output becomes an exhaustive set of exact bounded-reachability queries.
The command reports `SAFE` or `UNSAFE` and writes a complete counterexample trace
for an unsafe model. Static query-shape analysis rejects the specialized backend
for this workload and uses CDCL: this is
a real external validation of the portfolio's no-regression path, not a claimed
CQ-SAT/GCC speedup.

The bundled examples also include an input-driven
[Peterson mutual-exclusion protocol](examples/aiger/README.md), proved SAFE
through frame 100 for every scheduler and signal sequence, and an
[eight-bit SPI receiver](examples/aiger/README.md), reported UNSAFE with the
shortest 17-frame input/latch trace. These independently sourced models exercise
the scalable fallback on real protocol and hardware semantics.

The importer supports the original five-field ASCII `aag` and binary `aig`
formats, arbitrary primary inputs, declared latch initial values, multiple bad
outputs, symbols/comments, and a bounded resource envelope. Binary inputs use
implicit input/latch/AND literals and checked little-endian base-128 delta
decoding before passing through the same topology and semantic validator as
ASCII. Extended AIGER 1.9 property sections are not yet supported.

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

For partially specified initial states and future observations, the exact
preimage experiment composes BDDs over the initial frame:

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 2,4,8,16 100 200000 natural 707070 results/symbolic-preimages.csv
```

The numeric gate is a hard BDD-node limit. Exceeding it rejects the instance;
the implementation never substitutes an approximate answer.
Available calibration-free orders are `natural`, `reverse`, `evenodd`, and
`dependency`; the last is derived once from the local dependency graph.
`dependency-guard` adds a fixed pre-cycle node-growth projection and rejects
before the hard limit when exhaustion is imminent.
`hybrid` uses that guarded BDD path first and switches growth-guard cases to a
persistent exact CDCL solver, restoring complete admission without approximation.
`benchmark-checkpoint-cdcl` tests an exact BDD-prefix-to-Tseitin-CNF checkpoint;
the current encoding preserves semantics but is experimental and slower than the
full-CDCL control on measured cascade cases.
`benchmark-checkpoint-aig` replaces each BDD decision with structurally hashed
AND/inverter logic. It is also exact, but expands the measured cascade prefix and
is retained as a falsified compaction strategy.
`benchmark-checkpoint-lazy` encodes only the checkpoint cone initially, adds
earlier observation cones on demand, applies those observations directly to BDD
roots, and reconstructs the full prefix witness. This removes most checkpoint
nodes exactly, but does not yet provide a stable speedup over full CDCL.
`benchmark-native-bdd-theory` keeps the prefix BDD native, propagates unary and
pairwise checkpoint consequences into CDCL, and learns query-gated checkpoint
conflicts. It is exact, but higher-order conflicts dominate the measured cascade.
Rejected checkpoint states are greedily generalized to smaller BDD-proven
incompatible subcubes. This strengthens learning, but the current repeated-
conjunction minimizer costs more than the conflicts it removes.
The cached extractor uses prefix/suffix BDD conjunctions to test each checkpoint
literal once. It preserves identical explanations but does not yield a robust
runtime improvement, showing reconciliation is now the larger bottleneck.
The global-clause compiler enumerates the bounded checkpoint image once,
generalizes unreachable states into exact clauses, and installs them in the
suffix solver for every query. On the measured width-9 cascade this is the first
variant with phase and unseen-holdout speedups after compilation amortization.
Cross-family validation is mixed: 6 of 18 asymmetric rows beat full CDCL and five
amortize within 50 queries. Treat the backend as an admitted specialization, not
a universal replacement.
The benchmark generator also includes asymmetric `hub3`, `tree3`, and
`irregular3` transition families for evaluating structural ordering rules.
When a complete symbolic frame repeats, the compiler stores the transient and
cycle once and answers later observations by exact modular cycle lookup.

## Repository layout

- `src/main.rs`: solver, generators, benchmarks, and regression tests.
- `examples`: executable DIMACS and temporal-verification demonstrations.
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
