# Firmware and robotics research roadmap

This roadmap turns CQ-SAT/GCC into a sequence of falsifiable research cycles.
Items move forward only with exact agreement, reconstructed witnesses,
predeclared baselines, repeated trials, and claim-bounded documentation.

## Programme principles

- Firmware and embedded verification remain the primary domain.
- Robotics work starts from controller, sensor, actuator, scheduling, and
  bounded planning semantics—not from marketing analogies.
- Every specialised backend fails closed to maintained SAT/SMT baselines.
- No per-formula timing calibration is allowed in an admission decision.
- Negative results and rejected variants are retained.
- “Novel” means independently reviewed against the literature, not merely new
  to this repository.

## Research cycles

1. **Phase-preserving counterfactual interfaces — implemented experimentally.**
   Exact powered summaries preserve constant firmware input phases and show a
   robust 1.42x–7.00x median scaling curve on the bounded infusion-pump model.

2. **Symbolic input projection — first exact support-projection stage
   implemented experimentally.** Wide declared buses are projected onto the
   exact transition/property support. A 16-input mobile-robot controller with a
   two-input cone shows a robust 2.46x–10.74x median scaling curve. Dense-support
   predicate projection remains open.

3. **Proof-carrying interface composition.** Emit independently checkable
   certificates for leaf projection, relational composition, semantic no-op
   updates, and recovered traces.

4. **Firmware task and interrupt semantics.** Add bounded scheduling interfaces
   for interrupts, watchdogs, DMA, shared peripherals, priority inversion, and
   race-triggered safety properties.

5. **Word-level BTOR2 interfaces.** Preserve bit-vector arithmetic, counters,
   timers, memory indices, and saturating control laws before bit blasting.

6. **Assume/guarantee component quotients.** Compose independently checked
   contracts for drivers, control loops, communication stacks, and redundant
   monitors; identify which component invalidates a system guarantee.

7. **Robotics controller verification.** Apply word-level component quotients to
   sensor fusion modes, actuator interlocks, emergency stops, motion envelopes,
   and bounded mission controllers.

8. **Counterfactual repair synthesis.** Go beyond explaining a failure: compute
   minimal input-contract, guard, or state-machine changes that eliminate it,
   with the repaired design independently reverified.

9. **Real-time and probabilistic extensions.** Explore timed interfaces and
   bounded uncertainty only after exact Boolean/bit-vector evidence is mature.

10. **External validation and novelty review.** Compare against maintained
    model checkers and explanation tools on public and confidential partner
    designs; commission independent technical, security, and scholarly review.

The order can change when evidence falsifies an assumption. Cycles are not
marked complete by scaffolding or a single favourable benchmark.
