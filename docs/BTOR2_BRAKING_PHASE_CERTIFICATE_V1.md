# BTOR2 braking-phase certificate v1

## Status

This document freezes the first exact, source-bound phase-composition
certificate for a resettable accelerate, brake, and stop controller. It is an
experimental bounded verification backend, not a production-supported
interface or a scholarly novelty claim.

## Admitted source language

The producer accepts only a BTOR2 model with:

- one one-bit reset input;
- two same-width unsigned word states named by structure as velocity and
  position;
- one one-bit latched braking state;
- literal-zero initial and reset values for all three states;
- no constraints;
- a nonzero literal acceleration `a`;
- a nonzero literal braking threshold `b`;
- a nonzero literal deceleration `d`;
- control `braking OR velocity >= b`;
- velocity update `control ? max(velocity - d, 0) : velocity + a`;
- position update from the old velocity, `position + velocity`;
- braking latch update to `control`; and
- a bad property exactly equal to `position >= limit`.

Commuted additions and the commuted Boolean `or` are accepted. Different
update timing, signed comparisons, wraparound, constraints, additional state,
different guards, and different bad predicates are outside the language.

## Exact reachable relation

After any reset, define age `k` as the number of non-reset transitions. The
controller follows one canonical trajectory. An arbitrary reset schedule
therefore reaches exactly the set of canonical ages from zero through the
current frame.

The first braking frame is:

```text
m = ceil(b / a)
```

The peak velocity is `a*m`. The number of braking transitions is:

```text
q = ceil((a*m) / d)
```

The stopped state is reached at age `m+q`. Before `m`, position is the sum of
an increasing arithmetic progression. From `m` through `m+q`, it is the sum of
a decreasing progression. It is constant thereafter. All arithmetic is
checked in a wider integer domain and the backend declines the query if any
source word can wrap.

This prefix relation proves both reachability completeness and safety for all
reset schedules without enumerating the Cartesian product of position,
velocity, latch, reset history, and frame.

## Certificate and independent checker

Certificate v1 binds:

- the SHA-256 digest of the exact BTOR2 bytes;
- property and horizon;
- input and state node identifiers;
- word width and the three controller literals;
- the position threshold;
- switch and stop frames; and
- maximum velocity and position claims.

The producer uses direct triangular polynomial formulas. The checker reparses
the source through its own structural path, proves the switch and stopping
boundary inequalities, and recomputes both phase sums from their first and last
terms. It does not trust the producer's shape, phase boundaries, endpoints, or
safety decision.

The format is canonical UTF-8 with LF endings, fixed field order, canonical
decimal integers, no NUL bytes, and a 64 KiB cap. The horizon cap is one
billion. Every single-byte mutation and every truncation is tested to fail
closed through decode or semantic verification.

## Portfolio v3

The static order is:

1. braking-phase certificate;
2. coupled-motion curve certificate;
3. one-state word-region certificate;
4. unchanged explicit exact search.

There is no timing calibration or formula-specific learning. An inapplicable
shape or a curve that intersects the bad property passes the original source,
property, and horizon unchanged to exact search. Backend failures are surfaced
with backend context and are never converted into verification answers.
Explicit search retains its 256-frame resource bound, so an intersecting query
beyond that bound fails closed with a resource error rather than returning an
unsupported answer.

Portfolio v3 adds the `braking-phases` backend and
`braking-phases-exact-safe` reason. Version 1 search, word-region, and motion
artifacts remain self-identifying, decodable, and verifiable. Existing Rust
callers retain the additive `produce_with_observation` interface and the
original `produce` interface.

## Predeclared gates and evidence

| Gate | Evidence | Result |
|---|---|---|
| Exact both-answer boundary | SAFE at 255 and UNSAFE at 256; SAFE at 159 and UNSAFE at 160 | Pass |
| Near-neighbour rejection | Semi-implicit position update routes both answers to exact search | Pass |
| Producer/checker diversity | Polynomial producer versus boundary and first/last-sum checker | Pass |
| Reachable-prefix completeness | Every small-model explicit layer has exactly the certified reset-prefix ages | Pass |
| Maintained controls | Official BTOR2Tools parsing/simulation, Z3, pinned Bitwuzla 0.9.1 | Pass |
| Deterministic compression | 1,180,313 to 386 bytes and 453,342 to 386 bytes | Pass |
| Hostile input | Size, syntax, source drift, claim mutation, every truncation and byte mutation | Pass |
| Self-service routing | Six answer-balanced simulated acceptance cases | Pass |

The local 21-trial cost run records 610x to 1,558x faster median verification.
Timing is not a CI golden and does not establish a cross-platform guarantee.

## Closest prior art and claim boundary

The underlying ideas are established:

- bounded reachability for piecewise-affine systems has a substantial
  complexity and verification literature;
- discrete-time piecewise-affine reachable sets and Lyapunov overapproximations
  are established;
- acceleration, braking-distance invariants, arithmetic-series summaries, and
  resettable transition systems are standard;
- BTOR2 parsing, bounded model checking, and word-level SMT are established;
  and
- proof-carrying safety and independently checked invariants predate this work.

Relevant starting points include:

- Bazille, Bournez, Gomaa, and Pouly,
  [On the complexity of bounded time and precision reachability for piecewise affine systems](https://arxiv.org/abs/1601.05353);
- Adjé,
  [Overapproximating the Reachable Values Set of Piecewise Affine Systems](https://arxiv.org/abs/1506.02857);
- Teichrib and Schulze Darup,
  [Reachability analysis for piecewise affine systems with neural network-based controllers](https://arxiv.org/abs/2411.03834);
- Niemetz et al.,
  [BTOR2, BtorMC and Boolector 3.0](https://doi.org/10.1007/978-3-319-96145-3_32); and
- the official [BTOR2Tools](https://github.com/Boolector/btor2tools) and
  [Bitwuzla](https://github.com/bitwuzla/bitwuzla) implementations used by the
  external gates.

Certificate v1 is useful proof-carrying product engineering, but the current
evidence does not distinguish its core mathematics from a straightforward
combination of known recurrence acceleration, piecewise-affine reachability,
and invariant checking. A future novelty candidate must compose separately
sourced controller and plant contracts, preserve a non-Cartesian interface
relation across the component boundary, and beat straightforward product or
SMT baselines under independent review.

## Explicit limitations

This version does not cover signed coordinates, reverse motion, asynchronous
sensor updates, nonconstant acceleration, environmental uncertainty,
continuous mechanics, multiple independently chosen control inputs, component
contracts, or unmodified public robot firmware. The bundled models are
product-shaped fixtures. The self-service result is simulated, not partner
evidence.
