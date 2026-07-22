# OpenTitan AON dual-timer invariant-chaining experiment v1

Status: predeclared experiment on a pinned public embedded core. Not a
production release or novelty claim.

## Question

Can GCC prove one exact state invariant, use it to simplify a second guarded
recurrence, and share the resulting recurrence claims across several bounded
properties in one source-bound certificate?

The target is the unchanged OpenTitan AON timer core at production tag
`Earlgrey-PROD-M6`. The wrapper enables both the wake-up timer and watchdog. It
exposes wake-up interrupt, watchdog bark, and watchdog bite as three ordered bad
properties. This extends the prior watchdog-only experiment from one live
counter to the core's two timer paths.

## Pinned model boundary

The source and Yosys revisions remain those in the existing
`opentitan-aon-timer` provenance record. The new builder makes one additional
assumption explicit: all undefined flip-flop initial values are set to zero by
Yosys `setundef -zero -init`. For this model, that supplies the post-reset zero
initial value for the core's asynchronous prescaler state. The wrapper already
initialises both externally held timer counts to zero.

This is a post-reset verification boundary. It is not a claim about an
arbitrary power-on state, analogue reset behaviour, clock-domain crossing, or
the complete Earlgrey integration.

Pinned Yosys produces a 44-node semantic model with one reset input, three
states, three bad properties, no constraints, and maximum word width 64. GCC's
strict parser initially rejected the standard BTOR2 `redor` operation emitted
for the prescaler zero test. The parser now implements reduction-or with exact
word semantics and a dedicated regression.

## Observed structure

The generated model contains:

1. a 32-bit watchdog reset/add recurrence;
2. a 12-bit prescaler whose reachable value is always zero;
3. a 64-bit wake-up count whose increment guard is the prescaler-zero result;
4. watchdog predicates at thresholds 5 and 9; and
5. a wake-up predicate at threshold 7, gated by the same prescaler invariant.

This falsifies the simpler hypothesis that the public target is merely two
independent one-state counters. Exact support requires invariant chaining:
prove the prescaler is zero, simplify the wake-up transition and predicate
under that proved fact, then combine the wake-up and watchdog recurrences.

## Predeclared acceptance criteria

The follow-up mechanism is accepted only if all of these hold:

1. The producer and verifier independently reconstruct the prescaler invariant
   and both timer recurrences from the exact BTOR2 source.
2. One canonical artifact preserves ordered results and exact earliest bad
   frames for properties 33, 37, and 41.
3. Horizons 4, 5, 7, and 9 agree with exact bounded exploration: all SAFE at 4,
   bark UNSAFE at 5, wake UNSAFE at 7, and bite UNSAFE at 9.
4. A billion-frame query is answered without explicit layer construction when
   neither recurrence wraps.
5. Any unsupported guard, changed invariant, cross-coupling, wrap, malformed
   source, or incomplete member causes complete-query exact fallback or a
   fail-closed refusal, never a partial answer.
6. The verifier rejects source, invariant, recurrence, result, frame, witness,
   property-order, horizon, and route mutations.
7. Evidence size and verification work are compared with real separate
   certificates wherever the existing bounded baseline can answer. An
   unavailable baseline is reported as unavailable, not as a performance win.
8. Official BTOR2Tools, a maintained SMT solver, pinned Yosys regeneration,
   Linux resource controls, and the three-platform Rust API gate pass on the
   exact implementation commit.

## Implemented result

Predicate-set certificate and portfolio v3 add the statically selected
`invariant-chained-regions` route. The producer reconstructs the zero
prescaler invariant, the direct watchdog recurrence, the invariant-guarded
wake-up recurrence, and each ordered predicate from the source. The verifier
repeats that reconstruction from the supplied source and compares every claim;
it does not accept the encoded invariant or recurrence as an axiom.

The retained acceptance run covers five horizons:

| Horizon | Bark | Wake-up | Bite | Certificate bytes |
| ---: | --- | --- | --- | ---: |
| 4 | SAFE | SAFE | SAFE | 445 |
| 5 | UNSAFE at 5 | SAFE | SAFE | 454 |
| 7 | UNSAFE at 5 | UNSAFE at 7 | SAFE | 463 |
| 9 | UNSAFE at 5 | UNSAFE at 7 | UNSAFE at 9 | 472 |
| 1,000,000,000 | UNSAFE at 5 | UNSAFE at 7 | UNSAFE at 9 | 515 |

The billion-frame result is produced without constructing explicit layers and
only while both admitted additions remain non-wrapping. Existing predicate-set
v1 and v2 artifacts continue to decode and verify under their original static
selection contracts. Ten acceptance controls cover query omission, order and
horizon drift, source, invariant, recurrence, result, frame, witness and route
mutation, plus every truncation of the h9 artifact.

The committed evidence is
`results/opentitan-aon-dual-timer-acceptance-v1.csv`, reproduced by
`scripts/run-opentitan-aon-dual-timer-acceptance.sh`. Separate per-property GCC
evidence is reported as unavailable because the retained exact backend cannot
answer the input-dependent wake predicate. That is not counted as a size or
performance win.

## Remaining boundary

This local result does not yet close the complete experiment. Pinned official
BTOR2Tools parses the dual model and independently replays isolated zero-reset
witnesses for the bark, wake-up, and bite boundaries at frames 5, 7, and 9.
Maintained SMT agreement, Linux resource enforcement,
three-platform downstream API checks, and hosted reproduction remain required.
A valid same-width cross-coupled recurrence and an ill-typed cross-coupling are
both rejected in unit controls. The current verifier reconstructs source
semantics rather than trusting certificate claims. Official replay checks the
positive boundaries, but a maintained SMT comparison is still required before
describing the complete result as independently validated.

Multi-invariant reasoning, cone decomposition, recurrence acceleration,
multi-property model checking, and compositional certificates all have prior
art. The exact combined contract remains a candidate question requiring focused
prior-art comparison and independent expert review.
