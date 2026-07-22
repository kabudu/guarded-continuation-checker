# Proof-carrying BTOR2 bitblast v1

## Problem

The explicit-state BTOR2 backend refuses the six-channel symbolic OpenTitan PWM
`OutputHigh` query at horizon 2 under its frozen 20-million node-step policy.
A refusal carries no answer. In particular, the horizon-1 SAFE result cannot be
extended by assumption.

## Exact backend

The new backend bit-blasts every supported strict BTOR2 operation into bounded
CNF. It preserves word widths, initialisers, transitions, all-frame constraints,
inputs, and the selected bad property. The implementation covers bitwise and
arithmetic operations, variable shifts, unsigned comparisons, reductions,
conditionals, slices, zero extension, and concatenation.

SAFE results contain a Varisat-native UNSAT proof. Verification reconstructs
the complete CNF from the separately supplied source and checks the proof with
the separate `varisat-checker` implementation. UNSAFE results contain one
packed valuation for every frame. Verification replays constraints, transitions,
and bad-property activation using the existing word-level interpreter and
requires the recorded first bad frame in that trace.

The canonical wire format binds source digest, property identifier, horizon,
answer, witness or proof, and a SHA-256 envelope checksum. Decoding preflights
the two-MiB byte bound, 64-frame horizon, witness count, and one-MiB proof bound,
then requires byte-identical re-encoding. Every truncation and every single-byte
mutation of the retained 125-byte UNSAFE certificate is rejected. Representative
header, proof-body, checksum, mutation, and truncation attacks against the
215,259-byte SAFE certificate are also rejected; its proof parser has separate
step and byte bounds.

## Retained comparison

| Channels | Horizon | Result | Bad frame | Variables | Clauses | Proof | Certificate | Explicit state |
|---:|---:|---|---:|---:|---:|---:|---:|---|
| 2 | 1 | SAFE | none | 8,971 | 27,352 | 145,121 B | 145,222 B | agreed |
| 2 | 2 | UNSAFE | 2 | 13,456 | 41,027 | 0 B | 125 B | agreed |
| 4 | 1 | SAFE | none | 10,937 | 33,668 | 180,005 B | 180,106 B | agreed |
| 4 | 2 | UNSAFE | 2 | 16,405 | 50,501 | 0 B | 125 B | agreed |
| 6 | 1 | SAFE | none | 12,903 | 39,984 | 215,158 B | 215,259 B | agreed |
| 6 | 2 | UNSAFE | 2 | 19,354 | 59,975 | 0 B | 125 B | resource refused |

The six-channel horizon-2 witness is the key result: bit-blasting does not just
extend capacity, it prevents an incorrect interpretation of the prior refusal.
The exact firmware input sequence reaches a high channel observation at frame 2
and replays on the original BTOR2 semantics.

The negative result is equally important. At horizon 1, the proof-carrying SAFE
certificate is orders of magnitude larger than the explicit-state certificate.
Bit-blasting must therefore be selected by a static resource gate for workloads
outside the compact explicit regime. It is not a universal evidence reduction.

## Reproduction

```console
scripts/run-btor2-pwm-bitblast-probe-v1.sh /tmp/result.csv
scripts/check-btor2-pwm-bitblast-probe-v1.sh /tmp/result.csv
cargo test --release --locked --test opentitan_pwm_symbolic_class_api bitblast
```

## Remaining gates

- Integrate a static, calibration-free explicit/bitblast route into the class
  property portfolio without falling back on invalid evidence.
- Add a canonical outer portfolio format and frozen compatibility fixture.
- Cross-check a larger horizon and operator corpus against maintained solvers.
- Reproduce certificate identity on hosted Linux, macOS, and Windows.
- Measure generation, checking, peak memory, and packaging costs.

Bit-vector bit-blasting, SAT solving, and UNSAT proof checking are established
techniques. This is a required product capability and not a novelty claim.
