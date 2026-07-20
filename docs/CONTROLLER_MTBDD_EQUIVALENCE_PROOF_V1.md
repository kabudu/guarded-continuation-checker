# Controller MTBDD equivalence proof v1

Status: experimental, deterministic, source-bound, and available through the
public Rust library. It is not yet the default portfolio artifact.

## Purpose

The original MTBDD verifier evaluates every controller state and relevant input
assignment. That simple path checked 131,072 assignments for the public washing
controller and dominated fresh portfolio verification.

The proof-carrying path constructs one Boolean miter between the AIGER
controller and the MTBDD's joint next-state and selected-output function. The
producer emits a bounded Varisat-native UNSAT proof that no differing assignment
exists. A fresh checker reconstructs the miter from the supplied controller and
MTBDD, validates the source and artifact digests, and checks the proof before
using the MTBDD for plant composition.

Omitted controller inputs retain the existing MTBDD semantics and are fixed to
zero in the miter. The proof does not broaden or alter that environmental
assumption.

## Artifact

`GCCMEP01` encodes version 1, the controller source digest, MTBDD digest, CNF
variable, clause, and literal counts, the bounded proof, and a whole-artifact
SHA-256 trailer. The artifact is capped at 2 MiB; the embedded proof retains the
existing 1 MiB and 100,000-step limits. Decoding rejects unknown versions,
noncanonical dimensions, truncation, trailing bytes, and mutation.

`GCCMPF01` combines the proof with an ordered source-bound MTBDD plant batch.
Its verifier checks the equivalence proof once and then recomputes every plant
member result without exhaustive controller assignment replay.
Its `assignments_checked` observation is therefore zero. The 131,072
assignments are the equivalence scope represented by the miter, not an
iteration count performed by the proof verifier.

## Public-controller result

One release-build arm64 run, using medians of three verification trials,
recorded:

| Measure | Result |
| --- | ---: |
| Controller assignments represented | 131,072 |
| Equivalence CNF | 2,690 variables, 8,078 clauses, 21,509 literals |
| Raw proof | 242,496 bytes |
| Proof production | 29.38 ms |
| Exhaustive equivalence verification | 875.03 ms |
| Proof verification | 7.20 ms |
| Proof/exhaustive verification ratio | 0.008226 |

On this retained run, proof checking was about 121.6 times faster than exhaustive
assignment replay. The tradeoff is explicit: raw proof bytes are roughly 28
times the 8,549-byte six-member MTBDD plant artifact measured before proof
integration. Hosted replication and whole-process measurement remain required.

Reproduce the observation with:

```sh
cargo run --release --locked --example public_washing_controller_mtbdd_proof
```

The raw row is retained in
`results/controller-mtbdd-equivalence-proof-v1.csv`.

## Claim boundary

SAT miters, equivalence checking, and independently checked UNSAT proofs are
established techniques. This is a substantial local consumer-speed result and a
useful proof-delivery mechanism, not a novelty claim. Before portfolio admission
it needs hosted Linux replication, process-level resource measurements, a
versioned CLI contract, compatibility fixtures, and comparison with established
proof-carrying hardware tools on equivalent evidence scope.
