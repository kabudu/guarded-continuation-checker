# BTOR2 cross-channel trace composition experiment v1

Status: first source-bound structural-constant mechanism passes. Canonical wire
format, maintained temporal comparison, mixed exact fallback, file integration,
whole-process resources and portability remain open.

## Product question

Can GCC express and independently check bounded temporal relationships between
two authentic RTL channels, then reuse one exact proof only when a
source-derived channel-pair equivalence class makes that reuse sound?

The preceding trace-monitor experiment handles one Boolean channel at a time.
This cycle moves the observation boundary to a same-frame relation between two
channels, then applies the existing masked finite-history semantics to that
relation.

## Frozen first relation language

Version 1 observes exactly one of:

- `Equal`: the two selected Boolean channel observations are equal; or
- `Different`: the two selected Boolean channel observations differ.

The resulting Boolean observation is consumed by the existing version-1 trace
pattern:

```text
length: 1..=8
mask:   length significant bits
value:  length significant bits, with value & !mask == 0
```

The two channel indexes must be distinct and in range. Version 1 does not
accept arithmetic comparisons, more than two channels, internal nodes,
different input environments, or inferred assumptions.

## Sound reuse invariant

The independent checker must derive the existing source-bound channel
partition. A pair query may reuse a representative proof only when both
ordered endpoints map to the same ordered pair of verified channel classes.
Singleton pair classes use direct exact evidence. Swapping endpoints is not
silently canonicalised, even though equality and difference are symmetric,
because later relation versions may not be.

For every derived UNSAFE result, the witness must replay against the concrete
target pair. Invalid structural admission, relation drift, endpoint drift,
source drift, or a refused exact backend returns no logical answer.

## First mechanism slice

The first implementation slice must:

1. construct a canonical BTOR2 Boolean relation from two authenticated channel
   observations;
2. feed that relation into the existing prefix-valid trace monitor without
   changing retained single-channel bytes or semantics;
3. reject identical endpoints and invalid indexes before solving;
4. agree with a separately constructed direct exact model for both relation
   kinds and both answer classes; and
5. retain negative results if proof reuse does not reduce complete evidence or
   checking work.

The complete product cycle additionally requires a canonical pair-query
artifact, static aggregate preflight, exact fallback, target witness replay,
hostile codec tests, maintained Yosys plus Z3 comparison, whole-process
resources, cross-platform identity, and self-service file integration.

## First retained result

The fixed six-channel OpenTitan PWM probe applies equality and difference to
three ordered pairs drawn from the independently verified class `[0, 2, 4]`.
It covers a frame-zero control, a two-frame transition, and constant-low and
constant-high two-frame patterns through horizon 2.

| Logical queries | Members | Structural members | Reused queries | Structural bytes | Member evidence | SAFE | UNSAFE |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 24 | 8 | 8 | 16 | 460 B | 8 B | 15 | 9 |

The structural admission proves that each pair has identical Boolean traces
under the same input sequence. The checker therefore evaluates the equality or
difference trace as a constant, rather than asking a solver to rediscover that
fact. Every UNSAFE target is reconstructed and replayed against the concrete
pair.

The naive exact horizon-2 route exceeded the configured UNSAT proof-step limit,
and release-mode explicit search exceeded its node-step limit. This is a
positive governed-capability result for the structural route, not a speed ratio
against a successful baseline. The
[complete retained rows](../results/opentitan-pwm-channel-pair-trace-probe-v1.md)
also record the separate two-channel complement control and the remaining
comparison gaps.

## Claim boundary

Relational monitors, self-composition, symmetry reduction, bounded model
checking, SAT proofs, and representative proof reuse have substantial prior
art. Passing this experiment establishes a product capability, not scholarly
novelty. A novelty claim remains prohibited without a distinct invariant,
closest-implementation comparison, and independent expert review.
