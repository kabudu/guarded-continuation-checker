# BTOR2 channel history-monitor experiment v1

Status: authentic persistence probe is negative. No novelty claim exists.

## Product question

Can a small source-derived monitor carry the firmware configuration history
that a current input bit loses, then support independently checked
mixed-class channel relations with exact fallback for every uncovered frame?

The preceding guarded-relation experiment proved that no single positive
current input among the six OpenTitan class inputs establishes equality or
difference for the selected mixed-class pair through horizon 2. This
experiment changes the semantic object, not the search threshold: the guard is
a deterministic state machine over declared firmware write traffic.

## Frozen monitor language

Version 1 tracks exactly two configuration classes. Each class declares three
source-authenticated Boolean input boundaries:

- `write`;
- `enable`; and
- `invert`.

For class `c`, the monitor state is:

```text
seen_c'   = seen_c OR write_c
enable_c' = write_c ? enable_input_c : enable_c
invert_c' = write_c ? invert_input_c : invert_c
```

All six monitor bits start false, matching the reset configuration of the
selected public harness. Version 1 exposes exactly three guard states:

- `BothUnwritten`: neither class has observed a write;
- `SameTrackedConfig`: both classes have observed a write and their retained
  enable and invert values are equal; and
- `OppositeTrackedInvert`: both classes have observed a write, retained enable
  values are equal, and retained invert values differ.

These guards are fixed before running the cohort. Results must not add,
remove, negate or combine guard states to improve acceptance.

## Temporal convention

At frame `t`, a guard reads monitor state derived from frames before `t`.
Inputs at frame `t` update the monitor for frame `t + 1`. This matches BTOR2
state-transition semantics and prevents a same-frame write from being treated
as already committed configuration.

False-guard frames provide no relation evidence. A temporal query may use a
guarded relation only when every significant frame is covered by a verified
monitor state. Otherwise the unchanged exact pair route must answer the query
or GCC returns no logical answer.

## First implementation slice

The first slice must:

1. authenticate all six boundaries as distinct in-range source input bits;
2. append one canonical six-bit monitor without modifying source state or
   constraints;
3. build `guard AND NOT relation` for equality and difference;
4. independently parse the generated product model;
5. reject boundary aliasing, non-input nodes, out-of-range bits, identical
   channel endpoints and unknown monitor states; and
6. preserve the original source input environment and bounded horizon.

The monitor transition equations are public evidence. A later artifact checker
must reconstruct them rather than trust serialized monitor state.

## Predeclared cohort

The first cohort uses channels 0 and 1 of the authenticated six-channel
OpenTitan PWM family through horizons 0, 1, 2, 4 and 8. It evaluates all six
combinations of:

- three frozen monitor states; and
- equality or difference.

The cohort retains every SAFE and UNSAFE answer, earliest bad frame, encoded
certificate size and verification result. It also includes:

- endpoint swapping;
- one changed boundary bit;
- one aliased boundary;
- one false-coverage temporal pattern; and
- one exact-fallback query.

No minimum positive count is required. If all guarded relations fail, that is
the retained result.

## Baselines

Every claimed guarded answer must agree with:

1. a direct GCC product model containing the same monitor, relation and
   horizon, solved independently per claim;
2. pinned Yosys plus Z3 over the same source and monitor equations; and
3. the existing unguarded channel-pair exact route where it admits the work.

Any later composition comparison must include monitor construction,
certificate bytes, checking work, exact fallback and uncovered queries. There
is no predeclared performance threshold.

## Falsification conditions

The mechanism fails if:

- a generated monitor differs from the frozen equations;
- a guarded answer or earliest bad frame differs from a direct baseline;
- a false-guard frame becomes relation evidence;
- a same-frame input update affects the current guard;
- hostile boundary drift is accepted;
- an UNSAFE witness fails concrete source replay;
- uncovered work receives a logical answer without exact evidence; or
- complete checking work grows against independent direct queries after all
  monitor costs are included.

Negative results remain in the repository.

## First retained result

The complete 30-row OpenTitan cohort produces eight SAFE and 22 UNSAFE
answers. Every frozen guard and relation is UNSAFE by horizon 2. At horizon 8,
the two written-configuration guards each admit equality and difference
counterexamples at frame 7.

The canonical monitor, hostile boundary checks and independent certificate
verification pass. The semantic hypothesis does not: retained firmware
configuration omits operational phase and internal channel evolution. The
[complete retained rows](../results/opentitan-pwm-channel-history-monitor-probe-v1.md)
reject promotion into the proof portfolio. False-coverage composition and
exact-fallback packaging are not implemented because the prerequisite
persistent guarded relation failed.

## Claim boundary

History variables, monitor automata, product constructions, relational
verification, assume-guarantee reasoning and proof reuse are established. A
passing result would establish a proof-carrying product capability, not a novel
algorithm. Novelty remains prohibited without a distinct invariant,
closest-implementation evidence and independent expert review.
