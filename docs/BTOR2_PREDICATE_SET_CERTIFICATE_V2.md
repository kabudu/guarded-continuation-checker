# BTOR2 predicate-set certificate v2

Status: experimental additive Rust API and self-service CLI. Not a production
release.

## Purpose

Predicate-set certificate v2 answers several bounded safety properties over one
BTOR2 transition system while storing one exact recurrence claim. Unlike v1,
the shared route represents SAFE and UNSAFE members together and records the
earliest bad frame for every UNSAFE member. Unsupported models preserve the
complete ordered query through ordinary exact fallback.

The admitted language remains narrow: one live one-bit reset input, one live
word state, no constraints, exact reset-add or reset-saturating-add semantics,
and bad predicates recognised as equality or unsigned-greater-than-or-equal
comparisons with literals. The word recurrence must not wrap within the query.

## Exact shared semantics

For an admitted recurrence, the reachable state at a frame is determined by the
number of consecutive advance choices since the latest reset. The verifier
reconstructs the transition shape from the source, computes the maximum exact
recurrence index, and checks every predicate against that same index range.

For equality with literal `L`, a bad frame exists only when `L` is aligned with
`reset + index * delta` and its index is within the horizon. For unsigned
greater-than-or-equal with literal `L`, the earliest bad index is the ceiling of
`(L - reset) / delta`, clamped at zero, when it is within the exact range. A
saturating recurrence uses the same calculation with its source-derived
saturation endpoint.

An UNSAFE member carries `advance_prefix` as its compact witness kind. This is a
source-reconstructed input prefix containing no reset through the stated bad
frame. A SAFE member carries no witness. The verifier recomputes the result,
earliest frame, predicate, recurrence, source digest, and query binding. The
certificate does not ask the verifier to trust a producer-supplied answer.

## Static portfolio and failure contract

The route is deterministic and does not use timing, training, or per-formula
calibration:

1. Parse and recognise the source once for the complete ordered property set.
2. For two or more recognised members over one exact recurrence, emit
   `shared_exact_region`.
3. For a singleton or any unsupported member, preserve the complete query using
   the existing ordinary exact bounded portfolio.
4. If any ordinary member cannot be answered within its governed limit, fail
   the complete production request without returning partial answers.

The verifier reproduces this selection. A forced ordinary v2 artifact is
rejected when the shared route applies, and a forced shared artifact is rejected
when source reconstruction does not support it.

## Query, format and bounds

The external query binds exact BTOR2 source bytes, 1 to 64 strictly increasing
bad-property identifiers, and one common horizon. Omission, duplication,
reordering, substitution, and horizon changes fail closed.

- current certificate version: `2`
- current portfolio version: `2`
- supported certificate versions: `1,2`
- maximum members: 64
- maximum horizon: 1,000,000,000
- maximum complete encoded artifact: 64 MiB
- canonical UTF-8, LF-only text without NUL or trailing fields
- no-clobber, mode-0600 CLI publication on Unix

A shared v2 member is encoded as:

```text
member=BAD_ID:PREDICATE:LITERAL:RESULT:BAD_FRAME:WITNESS
```

For example:

```text
member=18:ugte:5:UNSAFE:5:advance_prefix
member=22:ugte:9:SAFE:none:none
```

Decode performs a canonical round trip before verification. Ordinary exact
members retain their own certificate versions and source bindings.

## Compatibility

The producer emits v2. The decoder and verifier continue to accept retained v1
`shared_region` and `ordinary_exact` artifacts under their original v1 static
selection and verification rules. A v1 artifact is not reinterpreted as v2.
The public unsuffixed version constants identify the current v2 producer;
explicit `*_V1_VERSION` constants identify the retained compatibility format.

## Rust API and CLI

The public API remains:

```rust
let produced = btor2_predicate_set::produce(source, &[18, 22], horizon)?;
let bytes = btor2_predicate_set::encode(&produced.certificate)?;
let certificate = btor2_predicate_set::decode(bytes.as_bytes())?;
let summary = btor2_predicate_set::verify(
    source,
    &[18, 22],
    horizon,
    &certificate,
)?;
```

The self-service commands remain:

```sh
guarded-continuation-checker btor2-predicate-set-version
guarded-continuation-checker check-btor2-predicate-set \
  INPUT.btor2 18,22 HORIZON OUTPUT.btor2-set-cert
guarded-continuation-checker verify-btor2-predicate-set \
  INPUT.btor2 18,22 HORIZON OUTPUT.btor2-set-cert
```

Discovery reports supported and current certificate and portfolio versions.
Production and verification rows report the artifact's actual version.

## OpenTitan acceptance

The pinned OpenTitan AON watchdog wrapper exposes bark threshold 5 and bite
threshold 9 over one counter. The retained v2 results are:

| Case | Answers | Shared v2 | Separate exact | Difference |
| --- | --- | ---: | ---: | ---: |
| Horizon 4 | both SAFE | 348 bytes | 598 bytes | 250 bytes smaller, 41.8% |
| Horizon 5 | bark UNSAFE at 5, bite SAFE | 357 bytes | 517 bytes | 160 bytes smaller, 30.9% |
| Horizon 1,000,000,000 | UNSAFE at 5 and 9 | 384 bytes | search limit | exact baseline unavailable |
| Scale horizon 1,000,000,000 | both SAFE | 384 bytes | 652 bytes | 268 bytes smaller, 41.1% |

The billion-frame UNSAFE row is a capability result, not a compression claim:
the ordinary bounded producer refuses both separate requests above its explicit
search horizon while the recognised recurrence yields exact earliest frames.
Maintained Bitwuzla endpoints independently confirm all SAFE, mixed, and
billion-frame UNSAFE boundary answers.

`scripts/run-opentitan-aon-predicate-set-acceptance.sh` regenerates both models,
reproduces four v2 artifacts byte for byte, verifies three retained v1 artifacts,
checks all available separate-certificate measurements, records the unavailable
bounded baseline, and exercises nine hostile controls.

## Prior-art boundary and limitations

Multi-property hardware model checking, bounded model checking, BTOR2, shared
symbolic exploration, proof certificates, recurrence reasoning, and compact
counterexample descriptions are established. This work does not claim those
ideas.

The narrower candidate contribution is one canonical, source-bound artifact
that shares an independently reconstructed exact word recurrence across ordered
SAFE and UNSAFE members, preserves earliest bad frames with compact witness
kinds, deterministically rejects forced downgrade, and retains exact complete-
query fallback plus v1 verification. No scholarly novelty claim is made until
the focused prior-art and external expert-review gates close.

This is bounded evidence for a recognised recurrence, not an inductive proof of
arbitrary RTL. The OpenTitan wrapper fixes one watchdog configuration and omits
the surrounding register generator, clocking, lifecycle system, wake-up timer,
and product integration. The certificate format has not received independent
expert review, and external production acceptance remains open.

Primary comparison points remain:

- [HWMCC 2011 multi-property track](https://fmv.jku.at/hwmcc11/results.html)
- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf)
- [Progress in Certifying Hardware Model Checking Results](https://fmv.jku.at/papers/YuBiereHeljanko-CAV21.pdf)
- [Certifaiger](https://fmv.jku.at/certifaiger/)
- [Finite-state proof-certificate framework](https://arxiv.org/abs/1507.08716)
- [Bitwuzla](https://bitwuzla.github.io/)
