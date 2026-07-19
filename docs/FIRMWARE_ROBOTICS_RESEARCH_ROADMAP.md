# Firmware and robotics research roadmap

This roadmap advances Guarded Continuation Checker and CQ-SAT through a sequence of falsifiable research cycles.
Items move forward only with exact agreement, reconstructed witnesses,
predeclared baselines, repeated trials, and claim-bounded documentation.

The authoritative gates for the programme's higher bar are the
[production-readiness](PRODUCTION_READINESS_GAP.md) and
[novelty](NOVELTY_GAP.md) gap registers. A bounded research release does not
close either register by itself.

## Programme principles

- Firmware and embedded verification remain the primary domain.
- Robotics work starts from controller, sensor, actuator, scheduling, and
  bounded planning semantics, not from marketing analogies.
- Every specialised backend fails closed to maintained SAT/SMT baselines.
- No per-formula timing calibration is allowed in an admission decision.
- Negative results and rejected variants are retained.
- “Novel” means independently reviewed against the literature, not merely new
  to this repository.

## Research cycles

1. **Phase-preserving counterfactual interfaces (implemented experimentally).**
   Exact powered summaries preserve constant firmware input phases and show a
   robust 1.42x–7.00x median scaling curve on the bounded infusion-pump model.

2. **Symbolic input projection (first exact support-projection stage
   implemented experimentally).** Wide declared buses are projected onto the
   exact transition/property support. A 16-input mobile-robot controller with a
   two-input cone shows a robust 2.46x–10.74x median scaling curve. Dense-support
   predicate projection remains open in the released portfolio. A bounded
   exact BDD prototype now handles 9–16 relevant inputs, powers relations,
   recovers traces, and agrees with persistent CDCL and external Yosys across
   three state-dependent controllers. Negative short-horizon rows define a
   static admission boundary; broader public and partner designs remain open.

3. **Proof-carrying interface composition.** Emit independently checkable
   certificates for leaf projection, relational composition, semantic no-op
   updates, and recovered traces.

4. **Firmware task and interrupt semantics (first exact event-contract slice
   implemented experimentally).** Strict named CNF now preserves non-cube
   interrupt, interlock, and recovery rules through bounded phase composition,
   CDCL agreement, and direct-AIG witness replay. The three-product performance
   result is negative at 1.09x to 36.20x slower than CDCL. Independently checked
   CNF relation/terminal proof primitives now verify in 0.261 to 1.051 ms, making
   certificate v3 feasible. A deterministic v3 artifact now binds source and
   contract, independently checks whole-contract composition, and covers both
   answer classes, with a retained 2.26x to 7.23x verification overhead against
   exact CDCL. All 68 completeness obligations and four aggregates also pass a
   maintained CaDiCaL plus DRAT-trim baseline. Release-candidate CLI/Rust API v1,
   timing-free portfolio admission, exact fallback, and a 60-row answer-balanced
   cohort are now implemented and pass the supported Rust 1.97 Linux path.
   Tagged compatibility, watchdog/DMA/shared-peripheral semantics, and public
   design evidence remain open.

5. **Word-level BTOR2 interfaces (strict semantic core and first phase
   certificate implemented experimentally).** The [v1 core](BTOR2_WORD_CORE_V1.md) preserves bounded
   bit-vector arithmetic, counters, timers, deterministic state updates,
   constraints, and bad properties before bit blasting. Arrays, memory indices,
   signed arithmetic and generic proof-carrying composition remain open. The
   [counter-phase candidate](BTOR2_COUNTER_PHASE_CERTIFICATE_V1.md) now binds
   and verifies a strict reset-or-affine recurrence. A static counter-trace
   portfolio preserves rejected supplied traces through bounded exact replay.
   Broader composition and exact fallback for bounded search remain open. A
   pinned Bitwuzla 0.9.1 gate now agrees on the candidate's endpoint formulas
   and rejects a tampered endpoint.

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

Release-path work now includes candidate
[reproducible Linux evaluation bundle v1](LINUX_EVALUATION_BUNDLE_V1.md): static
x86_64 musl, SPDX, deterministic provenance, offline replay, and a protected
attestation workflow. It is production hardening rather than an algorithmic
research result. The first [hosted signed candidate](../results/linux-evaluation-candidate-v1.md)
passes exact source, workflow, runner, SLSA, and SPDX verification. The macOS
distribution decision, tagged-release evidence, and independent evaluation
remain open.

The order can change when evidence falsifies an assumption. Cycles are not
marked complete by scaffolding or a single favourable benchmark.
