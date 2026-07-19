# BTOR2 counter-phase certificate v1

This experimental certificate compresses long repeated-control traces for a
strict class of firmware counters. It binds each result to the exact BTOR2
source and lets a separately callable verifier check phase endpoints without
replaying every transition.

It is a candidate research primitive, not a generic model checker, a safety
proof, a production API, or a novelty claim.

## Admitted model

Version 1 requires exactly:

- one 1-bit control input;
- one state word of 1 to 64 bits;
- a literal constant initial value;
- no constraints;
- `next = ite(control, reset_literal, state + delta_literal)` or the equivalent
  subtraction form; and
- a claimed bad property that has no input dependency.

The recognizer operates on the normalized source graph. It does not infer an
affine approximation from samples. Saturation, multiple states, multiple
inputs, nonliteral deltas, constraints, and input-dependent bad properties are
rejected.

## Certificate and verifier

A phase is `(input, length, start, end)`. For an advance phase, the verifier
checks:

```text
end = start + length * delta mod 2^width
```

For a nonempty reset phase, it checks `end = reset`. It also checks canonical
alternation, continuity between phases, the total horizon, source SHA-256,
recognized node identifiers and constants, and the bad property at the final
state.

The certificate accepts at most 4,096 phases, a horizon of at most one trillion
transitions, and 512 KiB of canonical LF text. The source retains the BTOR2 core
limits. Certificate output uses create-new semantics. After a partial write
failure, an incomplete file is retained rather than deleting a possibly
substituted pathname. The missing final status marker makes it unverifiable.

Example:

```sh
cargo run --release -- \
  certify-btor2-counter-phase \
  examples/btor2/watchdog-counter-v1.btor2 \
  13 '1:2,0:1000000003' /tmp/watchdog.cert

cargo run --release -- \
  verify-btor2-counter-phase \
  examples/btor2/watchdog-counter-v1.btor2 \
  /tmp/watchdog.cert
```

The example proves a bad endpoint after 1,000,000,005 transitions using two
phase records. It does not prove that no earlier bad state occurred, nor that a
bad state is unavoidable under every input trace.

## Cohort and prior-art boundary

The accepted cohort contains a resettable watchdog and a resettable actuator
position counter. A saturating timer is deliberately included as a rejected
near-neighbour. Official BTOR2Tools remains the syntax and simulation baseline.
CI builds maintained Bitwuzla 0.9.1 from exact commit
`8d1eb01093ae54d9b4586456b69c3bf31000a4c2`. Its SMT-LIB baseline confirms the
watchdog and actuator endpoint equations are satisfiable and rejects a tampered
watchdog endpoint as unsatisfiable. Bitwuzla documents that its BTOR2 input
support excludes model-checking features, so this comparison covers endpoint
word formulas rather than the sequential source model.

Closed-form acceleration of affine recurrences, run-length encoding, BTOR2,
SMT bit-vectors, and trace certificates are established ideas. The current
combination has not yet been shown to be novel. A credible contribution would
need a broader composition rule, independent comparative evidence, and proof
that the source-bound certificate provides a capability or assurance not
already available through straightforward acceleration and SMT encodings.

## Exact replay portfolio

The counter-trace commands apply a static two-backend rule:

```sh
cargo run --release -- \
  certify-btor2-counter-trace INPUT.btor2 BAD_PROPERTY PHASES OUTPUT.cert
cargo run --release -- \
  verify-btor2-counter-trace INPUT.btor2 OUTPUT.cert
```

The closed-form backend is attempted first. If its structural admission fails,
the unchanged source and phase trace pass to exact step-by-step replay. Replay
has its own certificate format, a 100,000-transition limit, and a combined
10,000,000 normalized-node-step limit. It supports the
one-bit-input BTOR2 core rather than pretending a rejected recurrence was
affine. If either backend cannot prove the claimed bad endpoint, the command
returns an error and publishes no answer.

The saturating timer near-neighbour demonstrates this path: closed-form
admission rejects its nested saturation condition, while exact replay reaches
and verifies state 255 after 255 transitions. This closes fallback integrity for
supplied one-input bad-endpoint traces only, not for safety or reachability
queries over all possible traces.
