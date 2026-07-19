<p align="center">
  <img src="assets/brand/logo-horizontal.svg" width="680" alt="Guarded Continuation Checker, powered by CQ-SAT">
</p>

# Guarded Continuation Checker

**Guarded Continuation Checker, powered by CQ-SAT**, is an evaluation-ready,
proof-carrying bounded verification platform for embedded firmware and RTL.

Guarded Continuation Checker (GCC) is the umbrella product and workflow. CQ-SAT
is its exact continuation-quotient engine: a calibration-free specialised
backend with persistent-CDCL fallback outside its validated structural regime.
The Rust package, library and executable use the product name:
`guarded-continuation-checker` on the command line and
`guarded_continuation_checker` in Rust source. See the
[brand and naming system](docs/BRAND.md).

The method processes variables in a fixed order, canonicalizes the residual CNF
after each Boolean choice, and merges prefixes only when their residual formulas
are exactly identical. A representative path is retained so satisfiable terminal
states reconstruct complete assignments.

## Status

This is an evaluation-ready research prototype, not a production-qualified or
certified product, not a general-purpose replacement for CDCL SAT solvers, and
not evidence that P = NP.

The project does not currently claim production readiness or scholarly novelty.
Those higher bars are tracked explicitly in the
[production-readiness](docs/PRODUCTION_READINESS_GAP.md) and
[novelty](docs/NOVELTY_GAP.md) gap registers.

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

The [counterfactual portfolio v1](docs/COUNTERFACTUAL_PORTFOLIO_V1.md) exposes a
single exact evaluation command over partial AIGER input transcripts. Its
timing-free structural gate selects the independently checked dense predicate
certificate backend only inside the measured regime and otherwise preserves the
query through persistent-CDCL fallback. It is a bounded research contract, not
a production interface.

The answer-balanced [predicate certificate cost experiment](docs/PREDICATE_CERTIFICATE_COST.md)
shows that raw predicate queries are competitive on the admitted cohort, while
certificate publication and exhaustive checking are currently much more
expensive than CDCL. All raw trials and sub-1-KiB artifact sizes are retained;
this is a checker-optimisation target, not a hidden positive benchmark.

The follow-up [proof-carrying relation experiment](docs/PREDICATE_PROOF_RELATION_EXPERIMENT.md)
replaces exhaustive one-step input enumeration with direct edge witnesses and
independently checked UNSAT completeness proofs. It improves relation checking
by 280.32x at 16 inputs, with a 20.9-KiB proof tradeoff. This is a certificate-v2
candidate primitive. Terminal safe-set proofs also preserve exactness, ranging
from negative easy-case overhead to a 26.20x constrained 16-input speedup. v1
remains the portfolio format.

The experimental [proof-carrying predicate certificate v2](docs/PREDICATE_CERTIFICATE_V2.md)
now packages both proof primitives, deterministic phase powers, terminal
evidence and the final trace into a bounded canonical artifact. Its independent
verifier avoids the BDD producer and exhaustive input enumeration. On the
answer-balanced cohort, v2 cuts the 16-input end-to-end check from 136.045 ms to
0.831 ms (163.71x), at the cost of a 52-KiB artifact and slower production. V1
remains the portfolio default while process-isolation, checker-diversity and
broader performance gates are still open. Its documented
[reliability boundary](docs/PREDICATE_CERTIFICATE_V2_RELIABILITY.md) covers
corrupted artifacts, structural proof preflight and fail-closed dependency
errors. The [external proof baseline](docs/EXTERNAL_PREDICATE_PROOF_BASELINE.md)
exports every completeness claim as canonical DIMACS and checks both the 40
individual obligations and four exact selector-guarded aggregates with pinned
CaDiCaL 3.0.0 and DRAT-trim. All verified, providing implementation-diverse
evidence without improving performance: aggregate external checking takes
33.702--57.671 ms versus 0.311--0.882 ms for native end-to-end v2 checking.
Whole-certificate checker diversity remains open.

Firmware automation can discover the frozen
[predicate CLI contract v1](docs/PREDICATE_CLI_V1.md) with
`predicate-cli-version`. Its single machine-readable line declares supported
certificate formats, the portfolio format, proof format and all primary v2
dimension and evidence limits. The contract also fixes argument order, exit
meanings, migration rules and a multi-release deprecation window.

Rust integrations can use the typed
[predicate Rust API v1](docs/PREDICATE_RUST_API_V1.md). `PredicateTool` discovers
and validates a compatible executable, invokes it without a shell, and exposes
typed v1/v2 production and verification with logical results separated from
operational errors. The current API intentionally preserves an out-of-process
boundary for resource governance; it is not yet an in-process verifier. Every
call now has a configurable deadline and bounded stdout/stderr with typed timeout
and output-limit errors. Observed API calls return metrics schema v1
with operation, duration, stream sizes, limits, exit status and a stable failure
class, plus canonical CSV output for build and fleet aggregation.

Unix API jobs now run in their own process groups and apply a configurable
file-size ceiling; a deadline ends and reaps the full group. Linux and other
supported non-macOS Unix targets also apply a configurable address-space ceiling
(2 GiB by default). macOS reports memory containment as unavailable while
retaining process-group, deadline, stream and file controls.

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

The second [exact symbolic input projection](docs/SYMBOLIC_INPUT_PROJECTION.md)
experiment admits up to 64 declared firmware inputs when static AIG support
analysis proves that at most eight affect the combined transition/property
interface. On a 16-input mobile-robot obstacle-stop regression projected to two
inputs, median end-to-end speedup scales from 2.46x at horizon 8 to 10.74x at
horizon 64, with complete-input witness lifting and fresh-CDCL replay. Dense
wide-support predicates remain an open boundary for the released portfolio.

The next [dense predicate quotient experiment](docs/DENSE_PREDICATE_QUOTIENT.md)
replaces explicit enumeration for 9–16 relevant inputs with a bounded exact BDD
predicate and composes its latch relations across repeated phases. The
16-input fixture compiles to 159 BDD nodes and reconstructs an exact 32-frame
trace. Predicate-query amortisation is negative at 1–10 reuses, positive at 100,
and reaches a 4.36x median workload speedup at 1,000 reuses. This is promising
prototype evidence, not yet a portfolio or novelty claim.
An external maintained-Yosys bounded-query check also agrees across ten trials;
its process-level timing is reported separately and is not treated as an
in-process solver comparison.

A broader state-dependent matrix now covers 9-input interrupt arbitration,
12-input actuator interlocks, and 16-input sensor fusion. Across horizons 8–64,
median end-to-end ratios against persistent CDCL range from 0.81x to 2.35x,
with exact Yosys agreement and original-AIG witness replay. The retained
negative low-horizon rows define the current admission boundary.

The bounded [event-contract experiment](docs/EVENT_CONTRACT_EXPERIMENT.md)
extends phase predicates from input cubes to strict named CNF. It exactly
preserves mutual-exclusion, priority, interlock, and recovery rules through
phase composition and concrete witness replay. All 30 release-mode rows agree
with a separately encoded exact CDCL control, but CQ-SAT is 1.09x to 36.20x
slower on the current three-product cohort. This is a semantic capability with
a retained negative performance result, not a portfolio or novelty claim.

The follow-up [proof-carrying event-contract primitive](docs/EVENT_CONTRACT_PROOF_EXPERIMENT.md)
rebuilds CNF-constrained relation and terminal completeness obligations without
trusting the BDD, checks native UNSAT proofs, and directly replays every claimed
witness. Across the same 9, 12, and 16-input cohort, median evidence checking is
0.261 to 1.051 ms with 7.8 to 33.9 KiB of proofs. This establishes certificate-v3
feasibility, not a frozen artifact, portfolio admission, or novelty claim.

Experimental [event-contract certificate v3](docs/EVENT_CONTRACT_CERTIFICATE_V3.md)
now binds the source AIG and original named contract to edge witnesses, checked
completeness proofs, independently recomputed phase powers, the final answer,
and an optional replayed trace. It deterministically covers both answer classes
across 40 cost trials. Verification takes 0.288 to 1.419 ms and is 2.26x to
7.23x slower than exact CDCL on these individual queries, so v3 is an assurance
artifact, not a speed claim or portfolio default.

The [external event-contract proof baseline](docs/EXTERNAL_EVENT_CONTRACT_PROOF_BASELINE.md)
exports v3 completeness claims to source-bound DIMACS and checks them with
pinned CaDiCaL and DRAT-trim. All 72 individual obligations and four aggregates
verified across both answer classes. This adds maintained proof-format and
checker diversity; it does not make v3 a stable or production-admitted API.

The first public RTL compatibility corpus is under
[`corpus/rtl/yosys-simple`](corpus/rtl/yosys-simple/README.md). It pins five
unmodified upstream Yosys sources and exercises twelve separately authored
SAFE/UNSAFE properties across two Yosys versions. This is reproducible public
evidence, not a substitute for confidential design-partner validation.

## GCC verification portfolio

```sh
./target/release/guarded-continuation-checker \
  benchmark-cq-portfolio watchdog4 9 137,1333,7777 50 10 200000 4141414 \
  results/local-watchdog-portfolio.csv
```

The static gate uses only transition density, dependency fan-out, width, declared
query-batch size, and assumption density. It never trial-solves candidate
backends. Dense models up to width nine with at least eight queries and narrow hub
models up to width seven with at least 128 queries use CQ-SAT when the batch
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
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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
limit. Linux, the supported production-evaluation host, also enforces a 2 GiB
address-space limit. macOS remains supported for development and records
`synthesis_memory_limit_kind=unavailable`; its process-tree, file-size, timeout,
and model limits still apply, but it must not be used as evidence of hard memory
containment.

Completed evidence bundles use the strict, SHA-256-bound
[RTL artifact schema v4](docs/ARTIFACT_SCHEMA_V4.md). Validate one before
retention or downstream processing:

```sh
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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
the bounded deterministic regime remain eligible for CQ-SAT; primary-input
or wider models are sent directly to an exact Tseitin-unrolled CDCL backend.

```sh
./target/release/guarded-continuation-checker \
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
CQ-SAT speedup.

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

The executable is `target/release/guarded-continuation-checker`.

## Real DIMACS evaluation

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-dimacs \
  examples/modular-demo.cnf 10000 10 results/local-modular.csv
```

The command reports structural admission, compilation cost, state and artifact
sizes, repeated-query performance against persistent Varisat, agreement, and
witness validity.

## Repeated-query experiment

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-reuse \
  banded-planted 100 4 98302 20000 results/local-reuse.csv
```

## Temporal phase experiment

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-temporal-phase \
  2,4,6 10,100,1000,10000 100 12 424242 results/local-temporal.csv
```

This controlled family holds a `width`-bit state constant across a `horizon` of
local CNF transitions. The benchmark reports both a dense quotient traversal and
an exact repeated-transition kernel against persistent Varisat.

## Recognized transition vocabulary

```sh
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,8,12 10,100,1000 100 12 24680 results/local-cones.csv
```

Semantic recovery costs roughly the sum of the local truth-table sizes rather
than `2^(2*width)`. The explicit jump kernel still contains `2^width` states, so
this removes one exponential factor but does not make unbounded-width SAT easy.

To avoid that explicit state table for deterministic trajectory queries:

```sh
./target/release/guarded-continuation-checker \
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
./target/release/guarded-continuation-checker \
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

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).

## Citation

See [CITATION.cff](CITATION.cff).
