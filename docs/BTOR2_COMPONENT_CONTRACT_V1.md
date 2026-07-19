# BTOR2 source-separated component contract v1

## Status

This document freezes the first exact source-separated controller and plant
contract supported by Guarded Continuation Checker. It is an experimental
bounded-verification interface, not a production-supported feature or a
scholarly novelty claim.

## Sources and synchronous wiring

The query has three independently supplied canonical files:

1. a BTOR2 controller;
2. a BTOR2 plant; and
3. a component contract.

The contract names three synchronous connections:

- one environment reset value drives both reset inputs;
- current plant velocity drives the controller's sensed-velocity input; and
- the controller's current combinational brake output drives the plant's brake
  input.

The controller state and plant state remain separate. GCC never generates or
trusts a merged BTOR2 source for component verification.

Contract v1 is canonical LF text with a fixed field order and a 4 KiB cap. It
binds exact node identifiers for both reset inputs, sensed velocity, braking
state, brake output, plant velocity and position, and the plant bad property.
The checker rejects missing nodes, extra inputs or states, mismatched widths,
constraints, an input-dependent plant property, noncanonical text, NUL bytes,
and unsupported versions.

## Specialised phase contract

The timing-free specialised backend admits the same mathematical controller
and plant relation as braking-phase certificate v1, but checks it across the
explicit component boundary:

- the controller independently owns the threshold guard and braking latch;
- the plant independently owns acceleration, saturating deceleration,
  old-velocity position update, and the position property; and
- the contract owns the feedback and control wires.

The producer discovers the split relation and emits a certificate binding the
SHA-256 digest of all three source files, horizon, widths, phase constants,
switch and stop frames, and maximum velocity and position.

The verifier reparses both sources, reparses the contract, independently
rebuilds every wire and source-side obligation, reconstructs phase boundaries
through boundary inequalities, and recomputes phase sums from first and last
terms. It does not call the producer's candidate recogniser and does not build a
monolithic product.

## Exact composed fallback

If the phase language is inapplicable or intersects the bad property, the
unchanged controller, plant, contract, and horizon go to exact composed search.
At each reachable pair of component states, the fallback evaluates both reset
choices, propagates plant velocity into the controller, evaluates the brake
wire, and steps both components synchronously.

An UNSAFE certificate carries a reset witness and is replayed through the
original two sources. A SAFE certificate carries every complete sorted layer;
the checker rebuilds both successors of every state and requires exact equality
with the next layer. Specialised failure is never converted into an answer.

Resource limits are:

- phase horizon: 1,000,000,000;
- search horizon: 256;
- states per layer: 65,536;
- total states: 262,144;
- estimated node steps: 30,000,000; and
- certificate bytes: 16 MiB.

An intersecting or unsupported query beyond the search horizon fails closed
with a resource error.

## Public interface and observability

The shell-free Rust module is `btor2_component`. The CLI commands are:

```sh
guarded-continuation-checker check-btor2-components \
  CONTROLLER.btor2 PLANT.btor2 CONTRACT.txt HORIZON OUTPUT.component-cert

guarded-continuation-checker verify-btor2-components \
  CONTROLLER.btor2 PLANT.btor2 CONTRACT.txt OUTPUT.component-cert
```

Production reports the stable reason `exact-phase-contract-safe` or
`specialised-inapplicable-or-intersecting`. Both operations report backend,
result, horizon, bad frame, logical state count, certificate bytes, and elapsed
microseconds. Timing never affects selection.

Certificates use canonical UTF-8 LF text, fixed field order, canonical decimal
integers, explicit backend tags, complete status, and strict size bounds. Phase
certificates are covered by every-prefix truncation and every-byte mutation
tests. Search certificates are structurally bounded, canonicalised by
round-trip encoding, and semantically replayed or checked for successor
completeness.

## Predeclared gates and results

| Gate | Evidence | Result |
|---|---|---|
| Source separation | Three exact digests and no generated monolithic source | Pass |
| Wiring integrity | Width, node, state/input vector and contract checks | Pass |
| Both answers | Three SAFE/UNSAFE boundaries agree with monolithic controls | Pass |
| Exact fallback | Semi-implicit SAFE and UNSAFE cases use composed search | Pass |
| Controller reuse | One unchanged controller source checked against two plants | Pass |
| Independent verification | Separate source matcher and phase arithmetic; exact fallback replay/completeness | Pass |
| Maintained controls | Official BTOR2Tools plus pinned Bitwuzla and monolithic portfolio | Pass |
| Hostile input | Source/contract/claim drift, canonical syntax and bounded decoding | Pass |
| Single-pair performance novelty | Component proof versus monolithic specialised proof | Fail |

The admitted component SAFE artifacts are 493 to 494 bytes and more than 99.8%
smaller than explicit monolithic layers. They are 107 to 108 bytes larger than
the equivalent monolithic specialised artifacts. Release-mode checking is 253x
to 1,089x faster than explicit-layer checking, but 1.35x to 1.38x slower than
the monolithic specialisation. Unsafe component witnesses are also larger
because they bind three sources.

The semi-implicit SAFE composed-search artifact is 36.01% smaller than the
monolithic explicit encoding, but this encoding result alone is not an
algorithmic novelty claim.

## Closest prior art and claim boundary

Source-separated composition is established. Relevant starting points include:

- Chen et al.,
  [Compositional Set Invariance in Network Systems with Assume-Guarantee Contracts](https://arxiv.org/abs/1810.10636);
- Ghasemi, Sadraddini, and Belta,
  [Compositional Synthesis for Linear Systems via Convex Optimization of Assume-Guarantee Contracts](https://arxiv.org/abs/2208.01701);
- Shali et al.,
  [Series composition of simulation-based assume-guarantee contracts for linear dynamical systems](https://arxiv.org/abs/2209.01844);
- Niemetz et al.,
  [Btor2-Cert: A Certifying Hardware-Verification Framework Using Software Analyzers](https://doi.org/10.1007/978-3-031-57256-2_7); and
- Yu et al.,
  [Introducing Certificates to the Hardware Model Checking Competition](https://doi.org/10.1007/978-3-031-98668-0_14).

Assume-guarantee reasoning, set invariance, simulation relations, component
interfaces, witness circuits, BTOR2 validation, and proof-carrying model
checking all predate this implementation. Splitting the files, binding three
digests, or checking known arithmetic across wires is not itself novel.

The single-pair performance hypothesis is explicitly falsified. The next
candidate question is whether one independently checked controller-local
obligation can be reused without rechecking across a batch of plant or firmware
variants, while preserving an exact interface relation and materially beating
both repeated monolithic checking and straightforward certificate bundling.

## Explicit limitations

The sources are product-shaped fixtures, not unmodified public robot firmware.
Contract v1 supports one feedback word, one control bit, and one shared reset.
It does not cover signed coordinates, disturbances, asynchronous scheduling,
sensor latency, continuous mechanics, multiple controllers, multiple plants,
contract refinement, proof caching, or cross-certificate reuse. The retained
self-service acceptance run is simulated, not partner evidence.
