# BTOR2 lagged channel-relation experiment v1

Status: authentic experiment is negative. No novelty claim exists.

## Product question

Can GCC prove a useful relation between authentic PWM channels after aligning
their observations by a source-derived temporal offset, instead of requiring a
same-frame equality or difference?

The preceding phase-abstraction experiment showed that the frozen ten-bit
phase and firmware-history vocabulary is non-functional for the same-frame
relation. This experiment changes the relation rather than adding
answer-selected state predicates.

## Frozen source derivation

The authenticated OpenTitan symbolic-class harness assigns phase delay zero to
even channels and phase delay two to odd channels. Version 1 therefore admits
exactly one non-zero lag:

```text
lag = odd_phase_delay - even_phase_delay = 2
```

The lag is derived from checked-in source before any solver result. It must not
be tuned, widened or selected per formula after evaluation.

## Frozen relation language

For channels 0 and 1, version 1 checks both temporal orientations:

```text
left_leads:  channel_0[t - 2] REL channel_1[t]
right_leads: channel_1[t - 2] REL channel_0[t]
```

`REL` is independently either equality or difference. The complete cohort
uses horizons 4, 8, 12 and 16. This produces 16 primary rows:

```text
2 orientations * 2 relations * 4 horizons
```

No row may be removed after results are known. Lag zero is retained only as a
same-frame control and cannot establish the lagged candidate.

## Prefix and non-vacuity invariant

A lagged relation is checked only at frames `t >= 2`. Frames before the lag
have no historical observation and must not be treated as satisfying the
relation.

Every generated model must contain:

- a two-frame shift register built from the selected leading observation;
- an explicit history-valid shift register;
- a bad property equal to
  `history_valid AND NOT lagged_relation`; and
- a separate coverage property showing that a valid comparison frame is
  reachable within the requested horizon.

A SAFE relation without verified coverage is `NONE`, not SAFE. Every accepted
result binds the source digest, semantic roots, endpoints, orientation,
relation, lag and horizon.

## Survival gate

The candidate survives only if one fixed orientation and relation is SAFE at
horizons 4, 8, 12 and 16, with verified non-vacuity at every horizon. A shorter
prefix result cannot authorize a longer or unbounded claim.

Every other orientation, relation, lag or horizon retains unchanged exact
fallback or returns no logical answer.

## First implementation slice

The first slice must:

1. authenticate both Boolean channel observations through the existing
   complete-region boundary;
2. reject identical endpoints, a zero lag and lags other than two;
3. construct a canonical two-frame observation and validity history;
4. emit separate coverage and relation models;
5. independently parse both generated BTOR2 products;
6. reject endpoint, orientation, relation, lag and horizon drift;
7. verify SAFE and UNSAFE evidence through the existing bitblast verifier; and
8. retain earliest bad frames and certificate sizes for the complete cohort.

Hostile controls include:

- endpoint substitution;
- endpoint reversal without changing the declared orientation;
- lag zero, one and three;
- a horizon shorter than the lag;
- relation substitution;
- missing coverage evidence;
- a relation certificate replayed against another horizon; and
- malformed or non-Boolean observations.

## Baselines

If the survival gate passes, compare the candidate with:

1. direct GCC bitblast checking of every lagged product;
2. pinned Yosys plus Z3 over identical shift and validity equations;
3. the existing same-frame channel-pair route; and
4. unchanged exact checking for every unsupported request.

All shift-register, coverage, proof, verification and fallback costs remain in
the comparison. There is no predeclared speed or byte threshold.

## Falsification conditions

The hypothesis fails if:

- any accepted implication relies on an invalid history prefix;
- coverage is absent or unverified;
- a result disagrees with direct exact checking;
- no fixed orientation and relation survives all four horizons;
- a hostile substitution is accepted;
- an unsupported request receives a logical answer without exact evidence; or
- complete candidate work exceeds direct checking after all construction,
  coverage and verification costs.

Negative results remain in the repository.

## First retained result

The complete 16-row cohort has verified history coverage at frame 2, but both
relations in both orientations have verified counterexamples at that same
first valid frame. All four horizons reproduce the result through monotonic
shortest-prefix checking.

The [retained result](../results/opentitan-pwm-lagged-channel-relation-probe-v1.md)
therefore rejects maintained-baseline and portfolio work. A source-derived
physical phase delay does not create a functional temporal relation while the
two firmware configuration classes remain independently symbolic.

## Claim boundary

Trace alignment, delay lines, retiming, sequential equivalence, product
automata and relational verification have substantial prior art. Passing this
experiment would establish a bounded product capability only. Novelty remains
prohibited without a distinct invariant, closest-implementation evidence and
independent expert review.
