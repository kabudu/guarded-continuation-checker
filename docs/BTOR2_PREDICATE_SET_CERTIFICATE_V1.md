# BTOR2 predicate-set certificate v1

Status: experimental additive Rust API and self-service CLI. Not a production
release.

## Purpose

The predicate-set portfolio answers several bounded safety properties over one
BTOR2 transition system without duplicating the same recurrence claim in every
SAFE certificate. It preserves the original ordered query and both answer
classes. It never converts an unsupported or intersecting predicate into a
logical answer.

The v1 shared language is deliberately narrow: one live one-bit reset input,
one live word state, no constraints, exact reset-add or reset-saturating-add
semantics, and bad predicates recognised as equality or unsigned-greater-than-
or-equal comparisons with literals. All other cases use the unchanged exact
bounded portfolio for every requested property.

## Query contract

A query contains:

- exact BTOR2 source bytes;
- 1 to 64 strictly increasing bad-property node identifiers; and
- one common bounded horizon.

The verifier receives these values separately from the certificate. It rejects
omitted, duplicated, reordered, substituted, or additional properties and a
different horizon. This avoids treating a producer-controlled certificate as
the authority for what the operator asked to check.

## Static portfolio

The route is deterministic and has no timing, training, or per-formula
calibration:

1. Independently attempt the existing exact word-region admission for every
   member.
2. Require at least two members, one identical source-bound recurrence claim,
   and SAFE disjointness for every predicate.
3. Encode the shared claim and choose it only if it is strictly smaller than
   the sum of the separate canonical word-region certificates.
4. Otherwise run the original `btor2_bounded` portfolio for every property.

The verifier reproduces the same decision and rejects forced downgrade. A
shared artifact is rejected if the current source and query no longer qualify.
An ordinary artifact is rejected if the shared route should have been used.

The ordinary route may contain a different exact backend for each member. For
example, one property can carry an UNSAFE explicit trace while another carries
a SAFE word-region proof. Failure of any specialised backend or exact fallback
fails the complete batch without returning partial answers.

## Shared certificate semantics

The shared certificate carries the source SHA-256, common horizon, reset input,
state, width, recurrence family and literals, maximum reachable recurrence
index, and an ordered list of bad-property predicate claims. The recurrence
claim occurs once. Each member adds only its property identifier, comparison
kind, and comparison literal.

For every member, the verifier reconstructs a complete region obligation and
passes it through the existing source-reparsing verifier. That verifier rebuilds
the transition shape and predicate from the BTOR2 graph, checks arithmetic
without word wrap, computes the reachable endpoint, and proves disjointness.
The shared verifier also requires all reconstructed reachability summaries to
agree.

The result is SAFE only when every member is proven disjoint through the common
horizon. There is no aggregated UNSAFE claim. Mixed or UNSAFE batches use exact
per-member evidence so the violated property and earliest bad frame remain
explicit.

## Canonical format and bounds

- `predicate_set_certificate_version=1`
- `PREDICATE_SET_PORTFOLIO_VERSION=1`
- maximum 64 members
- maximum 64 MiB complete encoded artifact
- canonical UTF-8, LF-only text with no NUL or trailing fields
- strictly ordered fields and members
- lowercase hexadecimal embedding for ordinary exact certificates
- no-clobber, mode-0600 CLI publication on Unix

Decode performs a round-trip canonicality check. The shared and ordinary
routes are self-identifying. Individual embedded certificates retain their own
version and source binding.

## Rust API

The additive public module is `guarded_continuation_checker::btor2_predicate_set`:

```rust
let produced = btor2_predicate_set::produce(source, &[18, 22], 4)?;
let bytes = btor2_predicate_set::encode(&produced.certificate)?;
let certificate = btor2_predicate_set::decode(bytes.as_bytes())?;
let summary = btor2_predicate_set::verify(source, &[18, 22], 4, &certificate)?;
```

`PredicateSetSummary` exposes the selected route, each property identifier,
backend, result and optional bad frame, SAFE and UNSAFE counts, and the checked
logical reachable-state count.

## CLI

```sh
guarded-continuation-checker btor2-predicate-set-version

guarded-continuation-checker check-btor2-predicate-set \
  INPUT.btor2 18,22 4 OUTPUT.btor2-set-cert

guarded-continuation-checker verify-btor2-predicate-set \
  INPUT.btor2 18,22 4 OUTPUT.btor2-set-cert
```

The result row includes the version, route, structural reason, ordered answers,
logical reachable-state count, certificate bytes, and elapsed observation. Time
is never an admission input and is not portable evidence.

## OpenTitan acceptance

The pinned OpenTitan AON watchdog wrapper exposes bark threshold 5 and bite
threshold 9 from the same unmodified production-tagged core. At horizon 4, the
shared certificate is 324 bytes versus 598 bytes for two separate certificates,
a 274-byte or 45.8% reduction. At horizon 5, bark is UNSAFE at frame 5 while
bite remains SAFE; the complete batch routes to ordinary exact evidence and
preserves both answers. A scale model proves both thresholds SAFE through
1,000,000,000 frames with a 360-byte shared certificate versus 652 bytes
separately, a 292-byte or 44.8% reduction.

The mixed artifact is larger than two loose certificates because it embeds
both exact artifacts in one canonical, query-bound envelope. That row validates
integrity and both-answer preservation, not compression.

`scripts/run-opentitan-aon-predicate-set-acceptance.sh` regenerates both models,
compares all three retained certificates byte for byte, verifies them, measures
separate evidence, and exercises seven hostile controls. Pinned BTOR2Tools
parses both multi-property models. Bitwuzla 0.9.1 checks the joint SAFE
endpoints and separate mixed-answer endpoints.

Hosted run
[`29789357950`](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29789357950)
reproduces this complete path on exact commit `448a758`. The green gates include
pinned Linux Yosys regeneration, the predicate-set acceptance report, official
BTOR2Tools, maintained Bitwuzla 0.9.1, Windows, macOS, and Linux downstream API
tests, dependency audit, the full retained test workflow, and reproducible
Linux packaging.

## Prior-art boundary

Multi-property hardware model checking is established and appeared as a
dedicated HWMCC track in 2011. Bounded model checking, BTOR2, shared symbolic
state exploration, proof certificates, Certifaiger, and composed witness
circuits are also established. This implementation does not claim any of those
ideas.

The narrow candidate distinction is a canonical source-bound certificate that
shares one exact word-recurrence claim across an ordered property set, selects
sharing only on deterministic encoded-size benefit, independently reconstructs
every member obligation, and preserves mixed answers through a downgrade-
detecting exact portfolio. No scholarly novelty claim is made until the
repository's prior-art and external-review gates close.

Primary comparison points:

- [HWMCC 2011 multi-property track](https://fmv.jku.at/hwmcc11/results.html)
- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf)
- [Progress in Certifying Hardware Model Checking Results](https://fmv.jku.at/papers/YuBiereHeljanko-CAV21.pdf)
- [Certifaiger](https://fmv.jku.at/certifaiger/)
- [A framework for proof certificates in finite state exploration](https://arxiv.org/abs/1507.08716)
- [Bitwuzla](https://bitwuzla.github.io/)

## Limitations

This is bounded safety evidence for a recognised recurrence, not an inductive
proof of an arbitrary RTL design. The OpenTitan wrapper fixes one watchdog
configuration and omits the surrounding register generator, clocks, lifecycle
system, wake-up timer, and product integration. The certificate format has not
received independent expert review. External production acceptance and the
broader novelty search remain open.
