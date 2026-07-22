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
   [Bounded search v1](BTOR2_BOUNDED_SEARCH_V1.md) supplies exact both-answer
   fallback for one-input, constraint-free models. The first
   [exact word-region certificate](BTOR2_WORD_REGION_CERTIFICATE_V1.md) proves
   complete reset-add and saturating reachable layers without enumeration, and
   its static portfolio retains exact search elsewhere. The first
   [coupled-motion curve](BTOR2_MOTION_CURVE_CERTIFICATE_V1.md) preserves an
   exact velocity-position relation without a Cartesian state product. Broader
   phase composition now includes the
   [resettable braking certificate](BTOR2_BRAKING_PHASE_CERTIFICATE_V1.md),
   which exactly joins accelerate, brake, and stopped regions under every reset
   schedule. General controller/plant contracts, multi-input composition,
   constraint semantics, and product validity remain open. The first narrow
   [source-separated component contract](BTOR2_COMPONENT_CONTRACT_V1.md) now
   binds controller, plant, and wiring independently and preserves exact
   composed fallback. It closes the source-separation primitive but falsifies
   single-pair performance novelty. Cross-certificate controller-proof reuse
   now has a source-bound
   [controller-obligation format and predeclared strong baselines](BTOR2_CONTROLLER_OBLIGATION_REUSE_V1.md),
   an exact compact batch, and a measured local win for fully admitted batches.
   Mixed fallback batches do not win, and public-product validity remains open.
   A pinned Bitwuzla 0.9.1 gate agrees on counter, motion, and braking
   boundaries. The first production-tagged public target now carries
   [OpenTitan's AON watchdog](OPENTITAN_AON_WATCHDOG_EXPERIMENT_V1.md) through
   pinned Yosys export, a compact billion-frame proof, exact unsafe fallback,
   deterministic certificates, and hostile controls. This is one configured
   watchdog path, so broader public-product and independent acceptance remain
   open.
   The next fallback boundary is now predeclared as
   [bounded search certificate v3](BTOR2_BOUNDED_SEARCH_V3.md). A pinned
   public PLIC gateway exposes five independent control inputs and is refused
   by the one-input search contract. V3 must retain complete packed transition
   and terminal valuations, preserve v1/v2 bytes, and pass an independent
   generated-model oracle before it can support the
   [public PLIC experiment](ROALOGIC_PLIC_GATEWAY_EXPERIMENT_V1.md).
   The next interoperability boundary is
   [bounded search v4](BTOR2_BOUNDED_SEARCH_V4.md) now preserves exact BTOR2
   environment constraints, admissible terminal valuations, and assumption
   dead ends without changing retained v1 through v3 evidence. A constrained
   public PLIC workflow agrees with maintained Yosys plus Z3 through horizon
   16. Hosted amd64 run 29872388711 reproduces the pinned model, retained
   evidence, maintained-tool baseline, Linux suite, and downstream API matrix.
   The next predeclared fallback boundary is
   [bounded search v5](BTOR2_BOUNDED_SEARCH_V5.md), which preserves
   small word-valued register and sensor inputs without flattening away source
   widths, while retaining every v1 through v4 artifact. Its local core and
   pinned Caliptra public-design validation pass, including maintained Yosys
   plus Z3 agreement. Hosted amd64 run 29874337371 closes every predeclared v5
   gate.

6. **Assume/guarantee component quotients.** Compose independently checked
   contracts for drivers, control loops, communication stacks, and redundant
   monitors; identify which component invalidates a system guarantee. The
   first exact controller/plant wiring contract and both-answer fallback are
   implemented. The first reusable controller-local obligation and compact
   plant-member batch are implemented for one controller family. Contract
   refinement, multiple interacting components, and blame assignment remain
   open.

7. **Robotics controller verification.** The first exact coupled-motion
   envelope and resettable braking-phase primitives are implemented. Extend
   them from the first separately sourced controller/plant contract to reusable
   local proofs, bounded control
   inputs, signed coordinates, actuator interlocks, asynchronous emergency
   stops, motion envelopes, and bounded mission controllers using unmodified
   public designs.

8. **Counterfactual repair synthesis.** Go beyond explaining a failure: compute
   minimal input-contract, guard, or state-machine changes that eliminate it,
   with the repaired design independently reverified.

9. **Proof-carrying dense controller relations.** Generalise the narrow braking
   obligation into a source-bound finite controller relation with witnessed
   edges and independently checked row-completeness proofs. Reuse it across
   multiple plant, bus, scheduler, or fault environments, while preserving exact
   fallback and trace reconstruction. The
   [v1 experiment specification](DENSE_CONTROLLER_RELATION_V1_EXPERIMENT.md)
   predeclares strong parse-once, proof-carrying, public-product, hostile-input,
   and cross-platform gates before implementation.

10. **Real-time and probabilistic extensions.** Explore timed interfaces and
   bounded uncertainty only after exact Boolean/bit-vector evidence is mature.

11. **External validation and novelty review.** Compare against maintained
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

The first controller/plant resource envelope now preflights conservative work
through public Rust, canonical file CLI, and typed process APIs without timing
calibration. Policy refusal now has a distinct versioned exit and typed failure
class. Local Linux correlation now preserves the direct-exact answer and typed
refusal under deadline, output, file, 64 MiB address-space, and process-group
controls. Hosted Linux reproduces the governed pipeline. An independently
sourced workflow remains required before using it as a constrained-workflow
guarantee.

The first six-job release-build pipeline now retains admitted, refused, and
invalid observations with exact per-job rows and a byte-stable aggregate. It
closes local aggregation and simulated self-service mechanics. Linux
process enforcement now passes locally for every governed verification in the
pipeline and is reproduced in hosted Linux CI. An independently sourced
constrained workflow and external suitability assessment remain open.

The next data-delivery cycle is predeclared as
[QatQ transport qualification v1](QATQ_TRANSPORT_QUALIFICATION_V1.md). It proves
whether exact QatQ compression can sit behind a GCC-owned, fail-closed
transport envelope for large proof-carrying revision batches. The experiment
passes hostile-input rejection, explicit resource limits, cross-platform byte
identity, Linux and macOS process resource measurements, semantic replay and a
retained zstd baseline. QatQ 0.1.4 now owns the exact-byte container and bounded
byte-chunk boundary, closing that promotion gate without changing the frozen
envelope.
Compatibility history and independent review remain open. It cannot change
certificate semantics, novelty status, or the `firmware-rtl-v1` support
boundary.

The next proof-delivery cycle now has a first
[governed proof-carrying MTBDD API](GOVERNED_PROOF_MTBDD_PORTFOLIO_V1.md).
It separately preflights the equivalence artifact and embedded UNSAT proof,
then preserves exact proof verification and plant composition under the
existing conservative work envelope. Canonical self-service policy and typed
process integration are now implemented. The first library portfolio selects
proof-carrying MTBDD or direct exact replay from the existing structural
admission result and rejects forced downgrade. Versioned portfolio file commands
and a strict typed process client now cover the admitted proof route, governed
proof refusal, and exact direct fallback. A deterministic six-job acceptance pipeline
now passes the public washing-controller proof route and exact fallback on macOS
and Linux, including typed refusal and hostile-input controls. Hosted Linux
reproduction and compatibility history remain open.

The pinned public controller and physical plant now have deterministic
source-to-model attestation through their exact Yosys revision, canonical
synthesis recipes, and byte-identical regenerated AIGER models. This closes the
public benchmark's local provenance mechanism gap. General signed partner build
provenance, independent synthesis equivalence, and compatibility history remain
open.

The [revision impact certificate v1](REVISION_IMPACT_CERTIFICATE_V1.md) cycle
now passes its predeclared OpenTitan PWM cohort gates. It provides a bounded,
deterministic certificate for counterfactual evidence invalidation and complete
inclusion-minimal semantic change sets across firmware revisions. A pinned
full-rebuild proof-producing baseline agrees on all 20 answers and independently
checks every artifact. GCC is faster at the matched shared-model orchestration
scope but produces a larger artifact. The controlled single-container baseline
removes per-job container startup and preserves a roughly 9.00 times advantage
against fixed four-way maintained orchestration, down from 53.78 times against
isolated containers. Prior work already establishes
incremental IC3 proof reuse, abstraction-precision reuse, mutation-impact
propagation, and hierarchical RTL lemma reuse, so the result remains a narrow
candidate contribution rather than a novelty claim. The isolated-container
bias gate is now closed without claiming a warm service because tool processes
remain fresh. The next experiment must test a larger connected public subsystem
and compare against a persistent maintained service if one is available.

The next larger public-subsystem cycle is predeclared as the
[OpenTitan PWM channel-family certificate v1](OPENTITAN_PWM_CHANNEL_FAMILY_CERTIFICATE_V1.md).
It tests whether one independently proved channel relation can be instantiated
across 2, 4, and 6 authentic repeated channels with explicit disjoint state
renaming, composed with one shared core, and checked against exact monolithic
and maintained proof-producing routes. This targets reusable repeated-IP
evidence, not another isolated timing comparison.

The first [preliminary channel-family probe](OPENTITAN_PWM_CHANNEL_FAMILY_PROBE_V1.md)
closes only the structural mechanism slice. Canonical source, parameter, root,
instance-map, proof, and exact-fallback artifacts now fail closed under explicit
resource limits and preserve both answers at 2, 4, and 6 instances. The compact
map is 77.69% to 84.18% smaller than its expanded relation, but the exact proof
members are unchanged and the complete family portfolio is larger than direct
exact evidence. This is a retained negative control. The authentic independent
channel fixture, representative-proof experiment, maintained baseline, process
resource evidence, portability, and self-service acceptance gates remain open.

The follow-up
[representative channel-orbit probe](OPENTITAN_PWM_CHANNEL_FAMILY_ORBIT_PROBE_V1.md)
passes the identical-binding mechanism gate. Five exact representative
certificates cover 10, 20, and 30 logical properties at 2, 4, and 6 channels.
At 6 channels this reduces retained exact evidence by 83.35% and the complete
artifact by 80.35%, while preserving every result and earliest bad frame over
five deterministic trials. This is established symmetry reduction, not a
novelty result. Authentic independent channel bindings, hidden-coupling
refusal, maintained evidence, process resources, portability, and self-service
acceptance remain open.

The source-faithful continuation is predeclared as
[authentic OpenTitan PWM channel extraction v1](OPENTITAN_PWM_AUTHENTIC_CHANNEL_EXTRACTION_V1.md).
It retains the exact upstream core, channel, and generated register package,
then asks whether complete repeated regions and mixed equivalence classes can
be derived and independently checked from the source-attested monolithic
model. This avoids treating the reduced behavioural boundary as product
evidence. Static refusal and exact fallback remain required outcomes.

The authenticated symbolic-class continuation now reaches
[property portfolio v2](OPENTITAN_PWM_SYMBOLIC_PROPERTY_PORTFOLIO_V2.md). A
static source-derived work projection chooses compact explicit-state evidence
or bounded proof-carrying bitblast evidence before solving. Verification
recomputes that route, independently checks each retained member, and replays
derived UNSAFE traces against their target channels. The six-channel horizon-2
workload closes its earlier refusal and reduces retained evidence plus
admission by 19.33% against direct witnesses. This remains established
symmetry and SAT proof engineering. Its canonical outer codec now supplies
bounded exchange, nested preflight and a frozen compatibility fingerprint. The
complete artifact is 4.53% larger than raw direct witnesses. Aggregate
production preflight now authenticates and plans the complete query batch and
refuses over-budget work before any property solver starts. A strict bounded
no-clobber file CLI now runs production and independent verification from a
canonical manifest and policy. Its typed governed process client now adds
strict discovery and result parsing, deadline and stream controls, process
containment, optional address-space enforcement, and refusal observations. The
next product boundary is measured process resources and phase observability,
followed by maintained-tool, realistic-property, portability, compatibility,
and independent-review gates.

The [bounded prior-art audit v1](PRIOR_ART_AUDIT_V1.md) materially narrows item
9. Compositional certification from BDD engines, arbitrary witness-circuit
composition, and shared multi-property checking already exist in the closest
published systems. The next experiment must therefore compare reusable
controller-local quotient evidence directly with a composed whole-circuit
witness. It must test whether a changed plant can be admitted and checked
without rebuilding controller evidence, while preserving both answers,
deterministic artifacts, low checker memory, and exact fail-closed behavior.
If that distinction does not survive the baseline, the algorithmic novelty
hypothesis is rejected and work continues as production engineering.

The order can change when evidence falsifies an assumption. Cycles are not
marked complete by scaffolding or a single favourable benchmark.
