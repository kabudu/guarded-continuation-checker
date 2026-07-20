# Proof-carrying MTBDD equivalence v1

## Outcome

GCC can now verify that a controller MTBDD is globally equivalent to its bound
AIGER controller without replaying every state and relevant-input assignment.
The producer constructs the Boolean miter, proves that no differing assignment
exists, and packages the UNSAT proof with the source-bound MTBDD. The verifier
reconstructs the miter and checks the proof independently.

On the public washing-machine controller and six physical-plant properties, the
proof-carrying artifact reduced median end-to-end verification time from
1.749 seconds to 0.870 seconds. The results agreed exactly. This is a 0.497
verification ratio, or approximately a 2.01x speed-up.

The gain costs storage and transfer. The compact exhaustive artifact was 8,549
bytes. The proof-carrying artifact was 251,221 bytes, or 29.39x larger. This is
therefore a fast-verification profile, not a universal replacement for the
compact artifact.

## Contract

The certificate binds:

- the SHA-256 digest of the controller source;
- the SHA-256 digest of the canonical MTBDD encoding;
- the exact CNF variable, clause, and literal counts;
- a bounded Varisat UNSAT proof for the reconstructed equivalence miter.

The miter covers every controller latch state and every declared relevant input
combination. Inputs omitted from the MTBDD boundary are fixed to false, matching
the existing exact MTBDD semantics. Its outputs include every next-state bit and
every selected observable controller output. A valid UNSAT proof establishes
that the AIG and MTBDD cannot disagree on any boundary assignment.

The plant result remains independently replayed. Only controller-equivalence
assignment enumeration is replaced by proof checking.

## Defensive limits

The decoder checks a canonical magic and version, SHA-256 payload integrity,
exact lengths, absence of trailing bytes, and fixed artifact limits. The miter
builder also caps CNF clauses and literals. Tests reject truncated encodings,
single-byte corruption, wrong source or MTBDD digests, invalid proofs, and
semantic MTBDD drift.

## Reproduction

```sh
./scripts/benchmark-controller-proof-mtbdd-plant.sh \
  target/controller-proof-mtbdd-plant.csv
```

The checked-in output is
[`results/public-washing-controller-proof-mtbdd-plant-v1.csv`](../results/public-washing-controller-proof-mtbdd-plant-v1.csv).

Timing is machine-dependent. Acceptance requires `answers_agree=true` and
`status=ok`. The speed ratio is evidence from this public workload, not a claim
that every controller or plant batch will improve.

## Interpretation

This is the first current GCC experiment to turn a global controller-equivalence
obligation into independently checkable evidence and improve the full public
controller-and-plant verification path. It uses established SAT proof checking.
The contribution being evaluated is the bounded, source-bound integration with
exact MTBDD controller reuse and plant-property evidence, not a new proof system.
