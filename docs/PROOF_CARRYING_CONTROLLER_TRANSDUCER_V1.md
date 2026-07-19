# Proof-carrying controller transducer v1

## Status

This document predeclares the first end-to-end dense controller obligation. The
underlying exact AIGER evaluation, outcome-completeness CNF, and independent
UNSAT proof checking exist as public library primitives. An in-memory
source-bound obligation, deterministic partition producer, and independent
verifier now exist behind a public Rust API. Canonical byte encoding, hostile
decoder testing, composition portfolio, and product gates do not yet exist.
This is not a novelty or production-readiness claim.

## Hypothesis

A bounded controller can be represented as a proof-carrying symbolic transducer
whose cells map:

1. a current controller state;
2. a cube over selected sensed inputs;
3. to an exact next controller state and selected actuator-output pattern.

The cube partition must cover every selected input pattern without overlap. A
cell may merge patterns only when every controller state has the same complete
next-state and output outcome. This retains input-to-output correlation that a
plain state relation would lose.

Each cell and source state carries:

- one concrete declared-input witness for its claimed outcome; and
- one independently checked UNSAT proof that no other next-state or selected
  output outcome exists inside the cell.

The verifier receives the original controller model separately, checks its
source digest, reconstructs every CNF obligation independently, verifies every
witness and proof, and checks partition coverage. It must not call producer
code.

## Static admission limits

- at most 16 selected sensed inputs;
- at most 4 controller latches;
- at most 4 selected controller outputs;
- at most 256 symbolic cells;
- at most 4,096 row proofs;
- at most 1 MiB per proof;
- at most 8 MiB total proof bytes; and
- at most 64 environment members.

Exceeding a limit rejects this backend before artifact publication. The
portfolio retains the unchanged exact query and routes it to exact fallback.

## Canonical partition

Version 1 uses the selected-input order declared in the boundary manifest. It
enumerates the bounded input space once, records the complete outcome vector
over all controller states, and builds a deterministic false-before-true
decision partition. A node becomes a leaf only when its complete outcome vector
is constant. Later versions may use a reduced decision DAG, but must beat this
version under the same proof and checking baselines.

## Predeclared gates

| Gate | Required result |
|---|---|
| Exact producer | Every leaf outcome agrees with direct evaluation for every input pattern and controller state |
| Complete partition | Cubes are canonical, pairwise disjoint, and cover the entire selected-input space |
| Independent verifier | Verification reconstructs CNF and checks witnesses and proofs without producer calls |
| Determinism | Repeated production yields byte-identical artifacts |
| Source binding | Controller digest and boundary manifest mutations fail closed |
| Hostile input | Every field, count, order, proof, witness, truncation, and size mutation fails closed |
| Exact composition | SAFE and UNSAFE answers and reconstructed traces agree with exact bounded model checking |
| Static fallback | Unsupported or over-limit cases preserve the original exact query |
| Strong artifact baseline | Shared transducer plus members beats ordinary proof-carrying per-member certificates |
| Strong checking baseline | Batch checking beats parse-once ordinary proof checking |
| Maintained tool | Yosys plus a maintained model checker agrees on every retained public case |
| Public product | One revision-pinned, unmodified embedded controller supplies at least two realistic environments |
| Cross-platform | Production and checking agree on Linux, macOS, and Windows |

## Falsification conditions

The experiment fails for a cohort if its exact canonical partition exceeds the
cell gate, if proof production exceeds resource bounds, or if strong artifact or
checking baselines lose. A fixture-only win, an over-approximated SAFE answer,
or per-formula timing calibration does not satisfy the hypothesis.

## Closest established techniques

Finite-state transducers, symbolic transition relations, BDD reduction,
assume-guarantee reasoning, proof-carrying model checking, and SAT proof logging
are established. The experiment claims none of those individually. Any later
novelty claim requires closest-prior-art review plus public evidence for the
specific source-bound reusable artifact and portfolio combination.
