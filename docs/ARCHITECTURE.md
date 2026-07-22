# Guarded Continuation Checker architecture

![Guarded Continuation Checker architecture: authenticated firmware and RTL inputs are routed through CQ-SAT or an exact fallback before independent evidence checking.](../assets/brand/platform-architecture.svg)

Guarded Continuation Checker is the platform and workflow. CQ-SAT is one exact,
specialised backend inside that platform. The architecture deliberately keeps
admission, resource governance, evidence publication and verification outside
the specialised solver path.

## 1. Reviewed inputs

The input boundary accepts supported firmware semantic slices, RTL designs and
transition models. A verification request also declares its assumptions, reset
policy, bounded horizon and named bad state. None of those declarations is an
unbounded whole-device safety claim.

## 2. Authenticated model boundary

The model boundary binds source identity, canonical model bytes and the ordered
query contract. Where a source-to-model attestation is required, the verifier
reconstructs or authenticates that mapping instead of trusting a caller-supplied
label. A certificate for one model or query cannot be replayed against another.

## 3. GCC orchestration

GCC preflights bytes, logical work and supported resource limits before proof
construction. A deterministic static gate selects a route without trial
solving or using observed timing. Atomic no-clobber publication prevents a
partial or collided artifact from appearing as a completed result.

Observability records operational evidence, but timing and memory measurements
cannot change a logical answer or force a specialised route.

## 4. Two exact routes

The specialised route uses CQ-SAT and related proof-carrying composition only
inside an admitted structural regime. It may merge equivalent residual
behaviour or reuse independently checked proof members, while retaining enough
information to reconstruct complete target traces.

The fallback route uses the supported exact portfolio, including persistent
CDCL, bit-blasting or explicit-state checking as appropriate. Unsupported
specialisation never becomes an approximate answer. A supported exact fallback
is used, or the request is refused before an answer is published.

## 5. Canonical evidence

Both routes converge on a canonical, integrity-bound artifact. Depending on the
query, it carries checked SAFE evidence or a complete UNSAFE witness together
with the authenticated model, selected route, resource policy and query
identity.

The artifact format is intended to make the result transportable and
independently challengeable. It does not make the producer part of the trusted
checker.

## 6. Independent verification

The separate verifier decodes canonical bytes, recomputes model and route
bindings, checks proof evidence, and replays witnesses against the supplied
source-bound model. It must reject mutation, truncation, drift, forced fallback
and resource exhaustion without returning a logical answer.

There are exactly three terminal classes:

- `SAFE · BOUNDED`: no declared bad state exists within the authenticated
  model, assumptions and horizon, backed by checked evidence;
- `UNSAFE · REPLAYABLE TRACE`: a complete counterexample reaches the declared
  bad state and replays against the model; or
- `REFUSED · NO ANSWER`: policy, capability, integrity or resource conditions
  prevent a verified answer.

`REFUSED` is not `SAFE`, `UNSAFE` or an inconclusive logical guess.

## Trust and claim boundary

The architecture reduces the trusted path around a bounded verification
result. It does not certify an entire device, establish that a source
translation covers behaviour outside its declared semantic boundary, prove an
unbounded property, or replace a regulated safety lifecycle. Those limitations
remain explicit in release notes, partner guidance and the
[production-readiness gap register](PRODUCTION_READINESS_GAP.md).
