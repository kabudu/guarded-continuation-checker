# Changelog

## Unreleased

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

## 0.1.0 - 2026-07-15

- Initial research release.
- Exact continuation quotient construction and witness recovery.
- Conservative structural gate and full frontier profile.
- Reusable assumption-query benchmark against persistent Varisat.
- Exact local clause insertion repair.
- Provenance-safe root rebuild for clause deletion.
- DIMACS/SATLIB evaluation path.
- Curated positive, negative, and corrected results.
