# Changelog

## Unreleased

## 0.12.0 - 2026-07-17

- Add strict RTL project config v1 with immutable include snapshots, bounded
  top-parameter overrides, declared clock/reset policy, and memory lowering.
- Add artifact schema v3 and firmware CLI v2 evidence for project semantics,
  plus a parameterised infusion-pump memory model cross-checked by SBY/Z3.
- Extend deterministic parser mutation coverage to strict project configs.

## 0.11.0 - 2026-07-17

- Establish RTL artifact schema v2 as the first compatibility-locked evidence
  contract and add a strict bundle validator for field, status, and snapshot
  consistency.
- Establish firmware CLI contract v1 with a machine-readable version query,
  fixed command signatures, and stable exit meanings.
- Bound direct ASCII AIGER ingestion to 256 MiB before and after reading.
- Add 20,000 stable-Rust parser and CLI mutation cases backed by persistent
  malformed and valid regression corpora.

## 0.10.0 - 2026-07-17

- Run Yosys in a dedicated Unix process group with a 512 MiB output-file cap,
  kill the complete group on timeout, and enforce a 2 GiB address-space limit
  on Linux.
- Record the effective containment platform and limits in safety reports and
  manifests, explicitly reporting memory containment as unavailable on macOS.

## 0.9.0 - 2026-07-17

- Add fail-closed, named RTL input assumptions that constrain every bounded
  frame, preserve their source artifact, and reject duplicates or unknown names.
- Cross-check constrained SAFE semantics independently with SymbiYosys and Z3
  while retaining the matching unconstrained UNSAFE regression.

## 0.8.0 - 2026-07-17

- Add a bounded multi-file RTL project safety gate with deterministic source
  staging, duplicate detection, aggregate limits, and manifest provenance.
- Remove stale source snapshots before atomic manifest publication so reruns
  cannot mix evidence from different project inputs.
- Mark RTL safety reports and manifests with artifact schema version 1.

## 0.7.0 - 2026-07-17

- Flatten hierarchical modules before RTL-to-AIGER export and make synthesis
  don't-care lowering explicit, enabling realistic multi-module controllers.
- Add an exact repeated-property BMC benchmark with bounded two-query reuse,
  cold-solver agreement, and a static no-regression portfolio gate.
- Add a five-module infusion-pump system and curated horizon-scaling evidence.

## 0.6.0 - 2026-07-17

- Add an RTL-to-safety-gate path that synthesizes bounded SystemVerilog through
  Yosys into the exact supported ASCII AIGER subset.
- Preserve Yosys input, latch, and bad-output names in human-readable traces;
  publish source, synthesis, model, solver, provenance, and manifest artifacts.
- Replace the hand-authored product workflow with safe and regressed
  SystemVerilog controllers cross-checked independently by SymbiYosys and Z3.

## 0.5.0 - 2026-07-17

- Add a product-shaped firmware safety gate with CI-specific exit statuses,
  GitHub Actions annotations, stable report artifacts, and a copyable workflow.
- Add safe and deliberately regressed infusion-pump controller models that show
  build acceptance and shortest-trace failure reproduction end to end.

## 0.4.0 - 2026-07-17

- Add exact primary-input and wider-model AIGER bounded model checking using a
  scalable Tseitin-unrolled CDCL fallback selected without trial solving.
- Combine all bad outputs and frames into one safety query, then minimize unsafe
  traces to the shortest bad horizon while preserving complete input witnesses.
- Add revision-pinned Peterson mutual-exclusion and SPI receiver models covering
  real SAFE protocol verification and UNSAFE hardware input-trace reconstruction.

## 0.3.0 - 2026-07-17

- Add a validated ASCII AIGER import path for closed deterministic safety models,
  including initial latch values and bad-state reachability queries.
- Add an independently sourced, revision-pinned four-bit counter-overflow model
  with its upstream MIT license and an executable portfolio workflow.
- Extend the static gate with query assumption density after the external model
  exposed a full-state-query counterexample to density-only admission.

## 0.2.0 - 2026-07-17

- Add the bounded, calibration-free CQ-SAT/GCC portfolio gate with exact
  persistent-CDCL fallback and declared-query amortization thresholds.
- Add unseen majority, multiplexer, and mixed-dynamics holdouts plus independent
  query-seed stability checks.
- Add executable watchdog/interlock and redundant sensor-voting verification
  examples showing specialized and fallback decisions.

- Add a bounded-width temporal model-checking phase benchmark.
- Add an exact repeated-transition kernel with full witness reconstruction.
- Preserve dense-quotient negative results alongside kernel measurements.
- Recognize a fixed deterministic transition vocabulary directly from layered CNF.
- Replace repeated template scans with one-pass normalization and logarithmic
  transition jump tables.
- Update GitHub Actions checkout to its Node 24 release.
- Recognize arbitrary total deterministic repeated transitions within a fixed
  width gate, including compositions outside the named rule vocabulary.
- Recover separable output functions locally from repeated CNF, eliminating the
  exhaustive current/next state-pair scan while preserving complete witnesses.
- Replay recovered local transition functions without an explicit `2^width`
  state table for fully specified deterministic initial states.
- Solve partial-initial-state temporal queries with exact BDD preimages and full
  witness reconstruction under a hard node-budget admission gate.
- Add calibration-free natural, reverse, even/odd, and dependency-graph BDD
  orders; preserve the negative symmetric-ring comparison and gated holdout.
- Add asymmetric hub, tree, and irregular transition graphs; dependency ordering
  reduces aggregate BDD size on phase and unseen holdout cohorts.
- Detect exact repeated symbolic frames and reuse transient/cycle checkpoints for
  long-horizon preimage queries without redundant BDD composition.
- Add an optional calibration-free BDD growth guard that rejects projected
  pre-cycle budget exhaustion early without approximating an answer.
- Add an exact hybrid backend that switches growth-guard cases from symbolic BDD
  preimages to persistent CDCL, restoring complete workload admission.
- Add an exact BDD-prefix-to-CDCL checkpoint experiment and preserve its negative
  performance result for naïve Tseitin encoding.
- Add a structurally hashed AIG checkpoint encoding and preserve the finding that
  it expands, rather than compacts, the measured cascade prefix.
- Add exact lazy observation-cone checkpoint encoding with direct BDD-root
  assumptions and prefix witness reconstruction.
- Expand cyclic symbolic frames correctly when checkpoint encodings reference a
  frame beyond the stored transient/cycle vocabulary.
- Add an exact native BDD-theory/CDCL bridge with activation-gated conflict
  learning and bounded pairwise theory propagation.
- Generalize rejected checkpoint states into exact BDD-proven incompatible
  subcubes and report learned-clause width.
- Add prefix/suffix conjunction caches for linear-pass exact BDD conflict
  explanation extraction.
- Precompile bounded exact global checkpoint-image clauses for reuse across all
  native BDD-theory queries and report recognition-inclusive break-even.
- Validate reusable global clauses across asymmetric hub, tree, and irregular
  transition families at widths 7, 9, and 11.

## 0.1.0 - 2026-07-15

- Initial research release.
- Exact continuation quotient construction and witness recovery.
- Conservative structural gate and full frontier profile.
- Reusable assumption-query benchmark against persistent Varisat.
- Exact local clause insertion repair.
- Provenance-safe root rebuild for clause deletion.
- DIMACS/SATLIB evaluation path.
- Curated positive, negative, and corrected results.
