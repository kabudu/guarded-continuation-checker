# BTOR2 channel phase-abstraction experiment v1

Status: predeclared. No result or novelty claim exists yet.

## Product question

Can GCC combine a shared operational phase with the frozen firmware
write-history monitor to derive non-vacuous mixed-class channel relations,
while keeping the abstract vocabulary materially smaller than the complete
channel state?

The configuration-history experiment fails because firmware state alone omits
operational phase and internal evolution. This experiment adds one
source-authenticated phase boundary. It does not expose arbitrary internal
nodes or select predicates after observing answers.

## Frozen abstraction

The source phase boundary is semantic root node 9 of the authenticated
six-channel OpenTitan PWM model. It is a four-bit counter exported as
`step_o`. The existing six monitor bits retain:

- whether each of two configuration classes has observed a write;
- the last written enable value for each class; and
- the last written invert value for each class.

The abstract vocabulary is therefore ten bits: four authenticated phase bits
and six deterministic monitor bits. The complete parsed source contains 33
state bits. The experiment fails its compression precondition if the
implementation adds another source-state discriminator or if the abstract
vocabulary reaches the complete-state size.

Version 1 freezes:

- phase values 0 through 8;
- `BothUnwritten`, `SameTrackedConfig` and `OppositeTrackedInvert`;
- equality and difference between channels 0 and 1;
- horizon 8; and
- the monitor equations and same-frame convention from version 1 of the
  history-monitor experiment.

No phase value, guard or relation may be removed after results are known.

## Non-vacuity invariant

Every accepted guarded relation requires two independently checked facts:

1. a reachability witness showing that the exact phase and history guard occur
   by the requested horizon; and
2. an exact proof or counterexample for
   `phase_guard AND history_guard AND NOT relation`.

A SAFE implication without a reachability witness is `NONE`, not SAFE.
Reachability and relation evidence bind the same source digest, boundaries,
monitor equations, phase value and horizon.

## Coverage invariant

One isolated phase fact is not a reusable temporal capability. The candidate
survives only if at least three consecutive reachable phase values establish
one relation under the same history guard. Any significant temporal frame not
covered by such a verified run requires unchanged exact fallback or returns no
logical answer.

This threshold is fixed before evaluation. It is a capability gate, not a
performance threshold.

## First implementation slice

The first slice must:

1. authenticate the phase node as an allowed semantic root with width four;
2. reconstruct the frozen six-bit history monitor;
3. build canonical phase equality and combined guard expressions;
4. emit separate reachability and relation bad properties;
5. independently parse both generated models;
6. reject phase-node, width, value, boundary, endpoint and horizon drift;
7. report the ten-bit abstraction and 33-bit complete-state control; and
8. refuse vacuous implications without a verified reachability witness.

## Predeclared cohort

The complete cohort contains 54 relation rows:

```text
9 phase values * 3 history guards * 2 relations
```

It also contains 27 reachability rows, one for each phase and guard pair. Every
row retains the result, earliest frame, certificate size, verification status
and refusal status.

Hostile controls include:

- a non-root phase node;
- a phase value outside the four-bit domain;
- a changed phase node;
- swapped endpoints;
- boundary substitution;
- a missing reachability certificate; and
- a relation certificate substituted across phase values.

## Baselines

If the three-consecutive-phase gate passes, compare the candidate with:

1. direct GCC bitblast products for every phase, guard and relation;
2. pinned Yosys plus Z3 over identical monitor and phase equations;
3. the unguarded exact channel-pair route; and
4. a complete-state lookup control that is disqualified as an abstraction but
   establishes the maximum available relation partition.

All proof, reachability, abstraction and fallback costs remain in the
comparison. There is no predeclared speed or byte threshold.

## Falsification conditions

The hypothesis fails if:

- any accepted implication is vacuous;
- a reachability or relation result disagrees with direct exact checking;
- fewer than three consecutive phases establish one relation under one guard;
- the abstract vocabulary is not smaller than complete source state;
- same-frame monitor semantics drift;
- hostile phase or certificate substitution is accepted;
- uncovered temporal frames receive an answer without exact evidence; or
- complete candidate checking work exceeds direct checking after all
  reachability and monitor costs.

Negative results remain in the repository.

## Claim boundary

Predicate abstraction, phase abstraction, history variables, monitor products,
relational verification, reachability certificates and assume-guarantee
reasoning have substantial prior art. Passing this experiment establishes a
bounded product capability only. Novelty remains prohibited without a distinct
invariant, closest-implementation evidence and independent expert review.
