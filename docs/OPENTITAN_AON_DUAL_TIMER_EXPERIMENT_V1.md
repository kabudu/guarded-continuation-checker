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

Pinned Yosys produces a 60-node text model with one semantic reset input, three
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

## Current boundary

After adding `redor`, the model parses and evaluates exactly. The existing
predicate-set v2 portfolio still refuses the complete query because it admits
only one-state recurrences and its ordinary search fallback does not support an
input-dependent bad expression. This is the intended pre-implementation
control. The next cycle must implement invariant-chained multi-recurrence
evidence without weakening the complete-query failure contract.

Multi-invariant reasoning, cone decomposition, recurrence acceleration,
multi-property model checking, and compositional certificates all have prior
art. The exact combined contract remains a candidate question requiring focused
prior-art comparison and independent expert review.
