# BTOR2 coupled-motion curve certificate v1

## Purpose

This experimental certificate proves bounded safety for two interacting word
states without enumerating their reachable pairs or forming a Cartesian product.
It targets a common discrete robotics pattern: velocity integrates constant
acceleration while position integrates the prior velocity, and one shared input
resets both coordinates.

The certificate is exact within its admitted source language. It is neither an
abstract over-approximation nor a floating-point kinematics model.

## Admitted recurrence

Version 1 requires exactly two same-width word states, exactly one Boolean
input, literal initial values equal to literal reset values, and no constraints.
For reset input `r`, velocity `v`, position `p`, and nonzero literal
acceleration `a`, both next-state expressions must be simultaneous:

```text
v' = if r then v0 else v + a
p' = if r then p0 else p + v
```

The distinction between old and updated velocity is part of the language. A
semi-implicit update `p + (v + a)` is rejected.

After `k` consecutive non-reset steps:

```text
v(k) = v0 + a*k
p(k) = p0 + v0*k + a*k*(k-1)/2
```

At frame `t`, every index from zero through `t` is reachable because the last
reset can occur at any earlier step. No other pair is reachable. The verifier
uses checked `u128` arithmetic and admits the proof only when both coordinates
fit the declared word width through the requested horizon.

The selected bad property must be the conjunction of `v >= velocity_limit` and
`p >= position_limit`, in either operand order. Both coordinates are monotone
inside the non-wrapping language. If the horizon endpoint does not satisfy both
thresholds, the complete bounded curve is disjoint from the bad set.

## Certificate and independent checking

The canonical certificate binds the exact source digest, horizon, bad property,
state and input identifiers, width, initial values, acceleration, thresholds,
and derived horizon endpoint. The checker:

1. parses the original BTOR2 independently of the producer artifact;
2. matches the two source next-state expressions using a checker-side path;
3. rechecks reset and initial equality;
4. rechecks the conjunctive source property and thresholds;
5. recomputes the endpoint using checked 3-by-3 affine matrix exponentiation,
   independently from the producer's triangular closed form; and
6. proves endpoint disjointness before returning `SAFE`.

The producer recogniser and checker matcher are separate paths. The producer
uses the triangular closed form while the checker uses logarithmic matrix
powering. They still share the strict BTOR2 parser, normalized model, primitive
checked-integer operations, and Rust crate. This is stronger algorithmic
diversity inside one process, not formal verification or a separately built
checker.

The existing bounded portfolio tries the motion curve, then the one-state word
region, then explicit exact search. A parser or resource-limit error is not
turned into an answer. Inapplicable, wrapping, intersecting, or near-neighbour
models pass the original query unchanged to explicit search.

This selection change increments the discovery-visible bounded portfolio from
version 1 to version 2. Version 2 continues to decode and verify every v1
one-state region and explicit-search artifact. The individual certificate
formats remain self-identifying and unchanged; only new production can select
the motion format.

## Limits and observability

- certificate version: 1;
- maximum horizon: 1,000,000,000;
- maximum certificate size: 64 KiB;
- canonical UTF-8 and LF, fixed field order, no NUL, canonical decimals;
- inherited 8 MiB and 100,000-node BTOR2 input bounds; and
- checked endpoint and logical-state arithmetic.

The self-service `check-btor2-bounded` and `verify-btor2-bounded` commands
report `backend=motion-curve`, answer, horizon, logical states, certificate
bytes, elapsed microseconds, and a deterministic selection reason. The public
`produce_with_observation` API exposes the same typed reason while the original
`produce` API remains compatible. Timing is observational and never affects
admission.

## Predeclared gates

1. Exact search and the portfolio must agree on both sides of two coupled-motion
   boundaries.
2. Each admitted SAFE artifact must be at least 99% smaller than its explicit
   complete-layer certificate.
3. A semi-implicit near-neighbour must be rejected and preserve both answers
   through explicit fallback.
4. Official BTOR2Tools must parse all three models and validate their concrete
   unsafe witnesses.
5. Pinned Bitwuzla must independently return UNSAT at both SAFE boundaries and
   SAT at both next-frame boundaries.
6. Every single-byte certificate mutation, every truncation, source drift,
   endpoint drift, recurrence drift, CRLF, and oversize input must fail closed.
7. The full Rust, Linux packaging, dependency, public RTL, and retained-cohort
   hosted gates must pass before merge.

The retained [six-row cohort](../results/btor2-motion-cohort-v1.md) closes the
local agreement and size gates. Merge remains prohibited until every external
and hosted gate passes.

The retained [release-mode verification-cost run](../results/btor2-motion-cost-v1.md)
records 333.48x and 805.73x median checking speedups over explicit SAFE layers
on one Darwin arm64 host. This is positive local evidence, not a portable
latency guarantee or replacement for hosted functional gates.

The retained [self-service acceptance run](../results/btor2-motion-self-service-acceptance-v1.md)
replays the documented production and verification workflow across the same
answer-balanced boundary set. It is simulated workflow evidence, not an
independent partner result or public robot validation.

## Prior-art boundary

Discrete kinematics, solving triangular affine recurrences, acceleration of
counter and affine systems, semilinear or polynomial reachability descriptions,
relational numerical domains, symbolic model checking, and inductive safety
certificates all have substantial prior art. Particularly close areas include:

- [Affine Extensions of Integer Vector Addition Systems with States](https://arxiv.org/abs/1909.12386),
  which studies affine counter updates and semilinear reachability relations;
- [Unbounded-Time Safety Verification of Guarded LTI Models with Inputs by Abstract Acceleration](https://doi.org/10.1007/s10817-020-09562-z),
  which concisely represents multi-step affine dynamics;
- relational polyhedral and octagonal program invariants;
- [Progress in Certifying Hardware Model Checking Results](https://doi.org/10.1007/978-3-030-81688-9_17),
  which uses witness circuits and independently checked inductive invariants;
  and
- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf),
  which establishes the word-level interchange and BMC setting.

The polynomial curve and its endpoint monotonicity are not novel. Nor is a
static exact fallback. The present result establishes a small, deterministic,
source-bound proof object that can be integrated into GCC without trusting the
producer or enumerating state pairs. Any future novelty claim must distinguish
a substantially broader composition contract from established affine
acceleration and certification methods, then survive external expert review.
