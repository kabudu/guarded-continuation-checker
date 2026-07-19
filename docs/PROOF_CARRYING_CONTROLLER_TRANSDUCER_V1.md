# Proof-carrying controller transducer v1

## Status

This document predeclares the first end-to-end dense controller obligation. The
underlying exact AIGER evaluation, outcome-completeness CNF, and independent
UNSAT proof checking exist as public library primitives. An in-memory
source-bound obligation, deterministic partition producer, and independent
verifier now exist behind a public Rust API. A bounded canonical binary codec
round-trips byte-identically and rejects every truncation and single-byte
mutation in the retained artifact test. Broader hostile corpora, composition
portfolio, and product gates do not yet exist. An exact sampled-control
composition API now verifies the controller obligation once, rejects
same-step sensor dependencies, explores bounded external plant inputs, and
reconstructs unsafe traces with the actual sensed controller input. An
independent direct-controller baseline agrees on every retained small query. A
timing-free portfolio selects a supplied verified artifact or direct exact
fallback when no artifact is available; invalid supplied evidence fails closed
instead of falling back. An opaque verified-controller handle and batch API now
verify the shared controller proofs once and expose aggregate member, answer,
proof-byte, reachable-state, and transition observations. Strong performance
and public-product baselines remain incomplete. This is not a novelty or
production-readiness claim.

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
- at most 6 controller latches;
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

## First batch-checking result

Run the predeclared parse-once comparison in an optimised build:

```console
cargo run --release --example controller_plant_batch_benchmark
```

The baseline independently verifies one complete canonical artifact per member,
so each artifact carries and checks the same source-bound controller proofs.
The shared path verifies one complete canonical batch artifact and checks those
proofs once. Across 101 interleaved trials, the retained one-bit controller
fixture produced these median ratios:

| Members | Shared/repeated checking time | Shared/repeated complete artifact |
|---:|---:|---:|
| 1 | 1.000 | 1.000 |
| 2 | 0.608 | 0.637 |
| 4 | 0.419 | 0.455 |
| 8 | 0.345 | 0.364 |
| 16 | 0.294 | 0.319 |
| 32 | 0.270 | 0.296 |
| 64 | 0.255 | 0.285 |

Every shared result exactly matched its independently checked counterpart. This
passes both mechanism-level reuse gates on the synthetic fixture, including
complete member results and source digests. It does not yet pass the
public-product gate. A repository-authored one-bit fixture cannot establish the
practical or novelty claim.

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
