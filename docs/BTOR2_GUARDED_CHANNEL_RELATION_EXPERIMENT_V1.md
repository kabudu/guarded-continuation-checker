# BTOR2 guarded channel-relation experiment v1

Status: first authentic probe is negative. No novelty claim exists.

## Product question

Can GCC independently certify a bounded relation between channels from
different structural classes only while a source-bound guard holds, then
compose those guarded facts into exact temporal answers without solving every
complete pair query again?

The preceding channel-pair experiment derives a constant equality or
difference observation only when both endpoints belong to one verified
structural class. This experiment deliberately targets mixed-class pairs where
structural identity alone is insufficient.

## Frozen first hypothesis

For one canonical Boolean guard `g`, two distinct Boolean channel observations
`left` and `right`, and relation `R` in `{Equal, Different}`, certify:

```text
for every admitted frame t through horizon h:
    g(t) implies R(left(t), right(t))
```

The guard must be selected from a source-authenticated Boolean input or
semantic root. It is not inferred from timing, solver behavior, trial runs or
the expected answer. The checker must reconstruct the guard and relation from
the separately supplied BTOR2 source.

Version 1 will admit only:

- one positive Boolean guard;
- equality or difference between two distinct Boolean channels;
- one shared input environment;
- a bounded horizon;
- explicit guard coverage for every composed temporal query; and
- exact fallback when the guarded certificate does not cover a frame.

Negated guards, arbitrary internal-node expressions, arithmetic relations,
unbounded induction and overlapping multi-guard selection are outside the
first language.

## Candidate certificate invariant

A guarded relation certificate is usable only if an independent checker
establishes all of the following:

1. the source digest, guard identity, endpoint identities, relation and horizon
   match the request;
2. the guard is a declared source-authenticated Boolean boundary;
3. every admitted guarded frame satisfies the claimed relation;
4. any temporal query answered without exact fallback is covered at every
   significant frame by a checked guarded fact;
5. UNSAFE results replay on the concrete target pair and original inputs; and
6. uncovered, ambiguous, malformed or resource-refused work returns no logical
   answer unless the unchanged exact route succeeds.

The certificate must never convert `guard = false` into evidence that the
relation is true. False-guard frames are uncovered and require another proved
guard or exact fallback.

## Predeclared cohort

The first cohort will use the authenticated OpenTitan PWM family and contain:

- at least one mixed-class pair with a valid input-conditioned relation;
- the same pair under a false or insufficient guard;
- an endpoint-swapped control;
- a changed-guard control;
- an adversarial trace whose significant frames cross the guard boundary;
- both SAFE and UNSAFE temporal answers; and
- at least one exact-fallback member.

If the current authentic PWM boundary cannot supply both a real positive case
and these controls without adding experiment-only semantics, the result is
negative and the experiment moves to another public source. The source will
not be altered merely to make the hypothesis pass.

## Baselines

Every retained result must be compared with:

1. a direct GCC product model that encodes the guard and pair relation for each
   query independently;
2. pinned Yosys plus Z3 over the same source, inputs, horizons and temporal
   semantics; and
3. the existing unguarded channel-pair workflow as a negative or fallback
   control.

The comparison reports logical agreement, earliest bad frame, complete
artifact bytes, proof construction work, checking work and whole-process
resources. There is no predeclared speed or size threshold.

## Falsification conditions

The hypothesis fails for this cohort if any of these occurs:

- a guarded answer disagrees with a direct exact answer;
- the earliest bad frame differs;
- a false-guard frame is treated as relation evidence;
- source, guard, endpoint, relation, horizon or input drift is accepted;
- target UNSAFE replay fails;
- preflight admits work that the declared production policy cannot complete;
- complete evidence or checking work does not improve on repeated direct
  queries after guard-certificate cost is included; or
- the closest maintained workflow provides equivalent reuse and evidence at
  the same scope.

A negative result remains part of the repository.

## First retained result

The fixed OpenTitan PWM probe checks one mixed-class pair under all six
positive firmware-input bits and both relation kinds through horizon 2. All
twelve claims are UNSAFE, with independently verified counterexamples at
frames 0 through 2.

The result rejects the version-1 guard language on this source. A current-frame
input bit does not summarize the configuration state accumulated from earlier
firmware writes. The complete
[retained rows](../results/opentitan-pwm-guarded-channel-relation-probe-v1.md)
therefore motivate a separately predeclared state-bearing guard monitor rather
than a wider search over answer-selected input bits.

## Claim boundary

Relational verification, product programs, self-composition, conditional
equivalence, assume-guarantee reasoning, symmetry reduction and proof reuse
have substantial prior art. Passing this experiment would establish a
source-bound proof-carrying product capability. It would not establish a novel
algorithm without a distinct invariant, a systematic closest-implementation
search and independent expert review.
