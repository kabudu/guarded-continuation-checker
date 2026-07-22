# OpenTitan PWM reachable-equivalence certificate v1

Status: proof-carrying admission mechanism passes; representative property
reuse is not yet integrated

## Question

Can GCC retain the bounded reachable channel classes found in the authentic
OpenTitan PWM family as a deterministic, source-bound artifact that an
independent consumer can verify without trusting producer labels?

## Contract

The v1 API accepts only deterministic, input-free, unconstrained BTOR2 models
with exactly one channel-to-aggregate observation per extracted channel. It
binds the artifact to:

- the exact source SHA-256;
- the ordered semantic roots;
- the expected channel count;
- the inclusive bounded horizon;
- every channel's local-state count, frame count, and trace digest; and
- a complete, canonical partition of channel indices.

The producer decides class membership by comparing complete vectors of local
state and observation values at every frame. It does not decide equality from
SHA-256. The digest is retained only as a compact trace identifier. The verifier
parses the source independently, re-extracts complete channel boundaries,
replays every transition, recreates every exact trace vector, and requires the
entire summary to match.

The wire format has a checksum, canonical ordering, bounded counts, a 1 MiB
static size ceiling, and an inclusive horizon ceiling of 4,096. Truncation,
every single-byte mutation, source drift, digest drift, class drift, malformed
partitions, noncanonical encodings, inputs, constraints, and excess horizons
fail closed.

## Retained result

The retained arm64 result is
[`results/opentitan-pwm-reachable-equivalence-certificate-arm64-v1.csv`](../results/opentitan-pwm-reachable-equivalence-certificate-arm64-v1.csv).
The checker fixes all semantic columns and deliberately treats microsecond
timings as observations rather than admission thresholds.

| Channels | Horizon | Exact classes | Reusable channels | Artifact bytes |
| ---: | ---: | ---: | ---: | ---: |
| 2 | 63 | 2 | 0 | 220 |
| 4 | 63 | 4 | 0 | 324 |
| 6 | 63 | 4 | 2 | 420 |

Only the six-channel source exposes reuse at this horizon: channels 2 and 4
share one trace, and channels 3 and 5 share another. Channels 0 and 1 remain
singletons. The smaller authentic models correctly produce no reusable class.

## Claim boundary and next gate

This result establishes deterministic, independently replayed admission
evidence. Bounded trace equivalence, symmetry reduction, cryptographic source
binding, and canonical certificates are established techniques. This cycle is
production-hardening evidence, not a novelty claim.

The artifact does not itself carry a property answer, and independent
verification deliberately replays every channel. The predeclared amortised
trace-predicate follow-up is now complete. It removes exactly one third of
logical predicate evaluations but does not improve end-to-end time. See
[the retained portfolio result](OPENTITAN_PWM_TRACE_PREDICATE_PORTFOLIO_V1.md).
The next gate must reuse an expensive independently checked property obligation
rather than another scan over materialised traces.
