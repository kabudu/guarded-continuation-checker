# Production-readiness gap register

Guarded Continuation Checker v0.28.0 is an evaluation-ready research prototype. This register is
the authoritative checklist for any future production-grade claim. A gate is
closed only by linked, reproducible evidence; passing unit tests alone is not
sufficient.

| Gate | Current evidence | Closure requirement | Status |
|---|---|---|---|
| Exact specialised backend | Certificate v1 independently rebuilds source support, phase powers and terminal sets; canonical candidate v2 checks relation/terminal proofs, phase powers and traces without exhaustive input enumeration across the three-product and multiphase cohort | Preserve both formats across compatibility changes; close v2 reliability and cost gates before portfolio replacement | Closed for bounded v1 and candidate-v2 semantics |
| Portfolio integration | Counterfactual portfolio v1 uses the versioned static rule; its strict consumer rechecks certificates or CDCL answers; forced resource exhaustion and both answer directions are covered across the three-product compatibility cohort | Preserve the contract and coverage across future API changes | Closed for bounded v1 |
| Resource governance | Static dimensions; Rust API applies deadlines, stream/file caps, Unix process groups and non-macOS Unix address-space limits; macOS group regression and a Rust 1.97 Linux container pass; [hosted run 29661860245](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29661860245) passes the Ubuntu limit regression, public RTL corpus and dependency audit; metrics record enforced controls and stable failure classes | Retain these regressions across supported releases; macOS remains development-only for hard memory evidence | Closed for predicate API v1 on supported Linux; macOS limitation explicit |
| Input reliability | Strict certificate/transcript sizes and syntax; v2 reliability boundary; structural native-proof preflight; 5,000 deterministic parser transformations; invalid-UTF8, sparse-oversize and oversized-proof tests; 128 proof transformations; canonical no-clobber DIMACS export plus 40 individual and four aggregate proofs accepted by pinned CaDiCaL/DRAT-trim under Linux process limits | Add continuous robustness coverage; extend checker diversity from completeness obligations to the whole certificate | Closed for bounded v2 parsing and external completeness checking; whole-certificate diversity open |
| Stable interface | Predicate CLI v1 freezes discovery, commands and exit meanings; typed Rust API v1 validates capabilities and exposes shell-free v1/v2 production/verification; a separate integration-test crate proves discovery, production and checking against the real binary | Preserve API compatibility across a tagged release; add an in-process verifier only if deployment evidence shows the process boundary is unsuitable | Closed for candidate CLI/Rust API v1; release evidence open |
| Observability | Portfolio/cost reports preserve backend decisions and timings; predicate Rust API metrics schema v1 records every observed operation's duration, stream sizes, configured limits, exit status and stable failure class; bounded aggregate schema v1 preserves operation, containment, and failure distributions; governed split observability adds four internal phase durations, eleven checked work counters, system-allocation events, and integrity-preserving process-local cache counters | Retain phase, resource, allocation, and cache acceptance across supported releases; allocator peaks remain covered only by the separate process-resource benchmark | Closed for the governed split candidate surface |
| Cross-platform distribution | Candidate Linux bundle v1 defines static x86_64 musl archives, SPDX 2.3 SBOM, deterministic provenance, non-executing offline verification, and two-source-path reproducibility; [signed candidate run 29675023822](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29675023822) passes exact SLSA and SPDX identity policy with [retained evidence](../results/linux-evaluation-candidate-v1.md) | Add a justified macOS distribution decision, tagged-release compatibility, and release-level verification evidence | In progress; first Linux candidate gate closed |
| Real product validity | Public synthetic/product-shaped fixtures plus one source-bound, production-tagged OpenTitan AON watchdog core under a narrow authored wrapper; pinned regeneration and both-answer acceptance pass | Multiple materially distinct unmodified public firmware/robotics designs at representative integration scope plus independent self-service evaluation outcomes | Open; first public embedded-core mechanism closed |
| Operational guidance | [Operations runbook](OPERATIONS.md), [isolation profile](ISOLATION_PROFILE_V1.md), and executable Linux qualification cover installation, sizing, failure handling, upgrade, rollback, incident response, restoration, and evidence retention | Retain the executable qualification and runbook drills across supported releases; independent operator execution remains part of the external-acceptance gate | Closed for the documented evaluation scope |
| Release governance | Claim-bounded tagged releases; the external production gate binds the exact register and release to an independently authenticated OpenSSH attestation | Production release checklist requires every row above closed or explicitly excludes the capability from production support; exercise the authenticated gate with real independent evidence | In progress; authenticated fail-closed mechanism complete |

## Event-contract experimental boundary

Event-contract v1 is not a production-supported interface. It has exact
agreement and replay evidence across three product-shaped fixtures, strict
parser bounds, explicit negative performance data, and independently checked
one-step/terminal proof primitives. Certificate v3 now adds a
deterministic format, source/contract binding, independent whole-contract
verification, both answer classes, a 1,000-input parser mutation corpus, and
proof swap/truncation rejection. Release-candidate event-contract CLI/Rust API
v1 adds typed discovery, bounded subprocess execution, stable metrics, a
timing-free structural portfolio, exact fallback, strict report replay, and a
60-row answer-balanced cohort. The six-case
[self-service acceptance run](../results/event-contract-self-service-acceptance-v1.md)
proves the documented workflow can be completed without formula calibration,
but it is simulated rather than independent evidence. The surface passes the
full Rust 1.97 Linux container suite locally and in
[hosted run 29667786512](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29667786512),
which also passes the public RTL corpus and dependency audit on the reviewed
commit. It does not yet have tagged compatibility history, public-design
evidence, or external acceptance evidence. These gates must close before event
contracts can change any production-readiness row above.

V3 completeness obligations now also pass a deterministic DIMACS export and a
maintained CaDiCaL plus DRAT-trim baseline across both answers. This removes one
checker-diversity gap, but the external tools are an evaluation harness rather
than a shipped runtime dependency and do not close the resource, compatibility,
portfolio, or acceptance gaps above.

## BTOR2 counter-phase experimental boundary

Counter-phase certificate v1 is not a production-supported interface. It
source-binds and separately verifies a strict one-state reset-or-affine
recurrence, passes accepted watchdog and actuator examples, rejects a
saturating near-neighbour, and agrees with pinned BTOR2Tools parsing plus
Bitwuzla endpoint formulas. It proves only a claimed bad endpoint for a supplied
phase trace. It does not prove safety, absence of earlier failures,
unavoidability, or generic BTOR2 semantics. The counter-trace portfolio now
preserves rejected supplied one-input traces through exact replay up to 100,000
transitions. Separately, bounded search v1 provides exact existential bad
reachability and its SAFE
  complement for the one-input, constraint-free subset through fixed state and
  work limits. Its explicit-layer certificate is a research reference, with no
  compatibility history, broad product corpus, or external acceptance evidence.
  Exact word-region certificate v1 compresses SAFE evidence for two recognised
  one-state recurrence families, with static explicit-search fallback and
  retained both-answer agreement. It remains a narrow source-language result,
  not evidence for interacting firmware state, memory, interrupts, or a public
  unmodified product design.
  Coupled-motion curve certificate v1 adds one exact two-state robotics
  recurrence, independent source matching, both-answer fallback, and retained
  artifact-size evidence. It does not cover signed coordinates, braking
  dynamics, multiple controls, sensor uncertainty, continuous plant behavior,
  or an unmodified public robot controller.
  Braking-phase certificate v1 adds a three-region resettable controller,
  deterministic portfolio v3 routing, complete hostile text mutation coverage,
  two product-shaped models, and independent BTOR2Tools plus SMT boundaries.
  It remains unsigned, discrete, and closed-loop within one source model. It
  does not cover separately supplied plant/controller components, external
  disturbances, asynchronous inputs, or a public robot design.
  Component contract v1 now binds separate controller, plant, and wiring files,
  exposes a shell-free Rust API and two CLI commands, preserves exact
  both-answer composed fallback, and retains an eight-case simulated acceptance
  run. It has only one feedback word, one control bit, and one shared reset. It
  lacks tagged compatibility, proof reuse, contract refinement, public-product
  evidence, independent acceptance, and operational deployment guidance.
  The controller-obligation v1 prototype is canonical, bounded, source-bound,
  independently checked, and exposed through Rust and CLI interfaces. It now
  drives a bounded exact mixed-answer batch with canonical artifacts and a
  normalized manifest workflow. Fully admitted local batches pass the strong
  artifact and checking baselines, while the 25% fallback control correctly
  records no efficiency claim. A static top-level portfolio now selects reuse
  only for at least two fully admitted members and retains ordinary exact
  certificates otherwise. Cross-platform packaging, maintained external
  controls, public-product evidence, and independent acceptance remain open.
  A six-case self-service acceptance harness proves the bundled workflow can be
  repeated without formula calibration, but it is simulated rather than
  independent evidence. Portfolio v3 exposes a stable typed selection reason
  and per-operation backend, artifact-size, logical-state, and timing
  observations; repository-wide multi-job aggregation remains open.
  It cannot change any production-readiness row until those gaps, stable release
compatibility, full hostile-input coverage, and public product evidence close.

## Proof-carrying controller transducer experimental boundary

Controller transducer v1 is not a production-supported interface. Its public
Rust API has bounded exact AIGER evaluation, complete next-state and selected
output obligations, independently checked UNSAT proofs, deterministic symbolic
partitioning, source binding, a canonical bounded codec, retained truncation and
single-byte mutation rejection, sampled plant composition, external-input
exploration, unsafe trace reconstruction, and a timing-free direct exact
fallback. Its complete canonical batch artifact binds plant sources and wiring,
recomputes all claimed results, and rejects every truncation and single-bit
mutation of the retained fixture. The local synthetic strong artifact and
checking baselines now pass. Supplied invalid evidence fails closed. Current
evidence is limited to small repository-authored controllers and plants. CLI
file integration, a broad generated hostile corpus, unmodified public-product
designs, maintained model-checker agreement, cross-platform replication,
release compatibility, and independent acceptance remain open. This capability
cannot change any production-readiness row until those gates close.

The first pinned public washing-controller candidate fails the controller
transducer admission gate with 1,028 canonical cells. It is retained as a
negative product-scale regression, not counted as public-product support.
The separate fixed-order MTBDD backend admits the same exact relation in 6,217
bytes and independently checks all 131,072 assignments. It does not yet compose
with representative physical product environments, expose a CLI file workflow,
or carry release compatibility, so it does not close a readiness row. Two
minimal appliance monitors now cover exact SAFE and UNSAFE composition,
direct-controller agreement, and maintained SymbiYosys plus Z3 agreement, but
remain repository-authored mechanism tests.
The public-controller complete-artifact reuse benchmark passes through 16
members, where shared bytes are 10.6% and checking time is 6.2% of repeated
artifacts. This closes the local strong-baseline mechanism gate only. Maintained
SymbiYosys/Yosys/Z3 now independently agrees on both minimal monitor answers and
the unsafe frame-10 result. This closes the maintained external agreement gate
for this narrow fixture only. Realistic physical environments, CLI integration,
compatibility history, and independent acceptance remain open.

The repository-authored stateful washing plant adds six physical state bits,
three nondeterministic disturbance events, six ordered safety properties, and
exact agreement for two SAFE plus four UNSAFE results. Its shared artifact is
78.6% smaller and checks 70.8% faster than repeated complete evidence, while
remaining at practical parity with checked in-process reuse in the retained
run. This closes a representative mechanism and stronger local-baseline gate.
It does not close real-product validity because the plant is not independently
sourced or physically calibrated. Hosted resource replication, compatibility
history, and acceptance remain open.
Pinned SymbiYosys with maintained Yosys and Z3 now reproduces the four exact bad
frames and two bounded SAFE results in one compiled formal session. This closes
maintained answer agreement for the disturbance fixture only. Independent plant
provenance and acceptance remain open.

An additional five-state-bit full-action plant checks all seven physical
controller actions against three expected-SAFE process properties through
horizon 64. It validates the expanded eight-output MTBDD boundary but remains a
repository-authored mechanism model and does not change a production gate.

The complete-cycle follow-up expands the fixed MTBDD action boundary to eight
and exercises seven fill-to-spin actions while preserving all other resource
caps. Three stateful process properties exactly match direct evaluation through
horizon 64. This closes the local complete-action-surface mechanism gate only.

Controller MTBDD CLI v1 adds machine-readable discovery, a canonical 64-KiB
source and query manifest, create-new production, independent verification,
exact per-member results, normalized paths, bounded no-follow file reads, and
hostile manifest, source-drift, artifact-mutation, and no-clobber regressions.
A six-member simulated self-service acceptance run reproduces all answers and
bad frames using only repository instructions. This closes local file
integration and simulated acceptance mechanism gates. The typed subprocess API
now applies existing execution controls, validates discovered limits and exact
result aggregates, and reports per-operation metrics. Portfolio fallback,
multi-job metrics aggregation, hosted cross-platform replication, tagged
compatibility, source-to-model attestation, calibrated plant validity, and
independent acceptance remain open.

The first whole-process resource comparison closes the local peak-memory
mechanism gap. On one arm64 development host, fresh GCC verification is 2.67
times slower but uses 85.2% less peak RSS than the maintained formal oracle.
Hosted Linux replication, phase-level attribution and independent measurement
remain open, so no production-readiness row changes.

Phase-level portfolio observations now separate bounded input loading, artifact
handling, semantic replay, publication, and total command time. The public
admitted batch shows replay dominates while loading is negligible. This closes
local phase attribution, not multi-job aggregation, hosted comparability, or
the controller-replay optimisation gap.

The library-level controller MTBDD plant portfolio now preserves valid bounded
queries through exact direct fallback for three explicit static resource-limit
rejections. Malformed models, semantic errors, query drift, and forced
downgrades fail closed. This closes the local fallback-integrity mechanism gap.
Hosted compatibility, independent acceptance, and source-to-model attestation
remain open, so no production-readiness row changes.

The proof-carrying MTBDD experiment removes exhaustive assignment replay from
one local checker path and provides canonical, bounded, source-bound proof and
plant-batch artifacts. Its public-controller in-process result is strong, but
the new versioned CLI and typed process client close local integration and begin
compatibility history. Retained public self-service acceptance now covers exact
answers, bad frames, drift, mutation, and no-clobber behavior. Hosted process
measurements reproduce the speed direction and negative verification-memory
tradeoff. Equivalent-certificate external comparison remains open. It stays
outside the production portfolio and changes no readiness row.

The first release-build whole-process baseline retains exact agreement and a
2.00x median proof-verification improvement on one arm64 host. This closes the
local process-overhead uncertainty. The resource companion records higher proof
peak RSS, so the profile is not an unconditional operational win. Hosted
Linux replication is retained; identical-scope maintained-tool evidence remains
open at equivalent certificate scope. An identical-query local baseline is now
retained and is negative on runtime against SymbiYosys/Yosys/Z3. Hosted Linux
reproduces that direction and the lower verifier-memory result.

Equivalent-certificate evidence remains the next proof-profile gate. The
[predeclared Certifaiger plan](CERTIFAIGER_EQUIVALENT_EVIDENCE_PLAN.md) requires
competition-standard SAFE witness and UNSAFE trace checking before any portfolio
or novelty conclusion.

The local arm64 equivalent-certificate qualification now passes for the six
frozen horizon-32 exports. Independent replay preserves the four shortest bad
frames and two SAFE answers, rIC3 emits both evidence classes, Certifaiger
accepts the SAFE witnesses, and `aigsim` accepts the UNSAFE traces. Hosted
amd64 qualification now reproduces the semantic results, all seven hostile
controls, all six external witness digests, and GCC's exact batch-proof digest.
The comparison is negative on GCC runtime, evidence size, and executable
footprint on both hosts, but positive on producer and verifier memory. The
amd64 verifier uses 7 MB versus 115 MB for the standard consumer. This closes
the comparison's hosted and deterministic-evidence gates. Integration policy,
formally verified checker comparison, constrained-device acceptance, and
external expert review remain open. This is evidence for a narrow low-memory
consumer profile, not for broad production superiority.

Controller/plant resource envelope v1 now exposes caller-selected artifact,
member, horizon, product-state, and conservative transition-evaluation limits
through the public Rust API. Both exact portfolio routes pass at their inclusive
boundaries, and each tighter policy fails before semantic replay. This begins an
explicit policy layer for the low-memory consumer profile. Canonical CLI policy
files, strict capability and result parsing, hostile policy rejection, and a
typed bounded process client are now implemented. Policy refusal has a separate
exit code, five versioned reasons, no logical answer, and a typed metrics class.
The release-build acceptance pipeline now retains six process jobs: two
verified, two valid policy refusals, one malformed policy, and one corrupt
artifact. Its byte-stable aggregate preserves 3 SAFE and 5 UNSAFE results from
verified jobs only and records every negative row. This closes local multi-job
aggregation and simulated self-service acceptance for the controller/plant
resource surface. Independently sourced workflow evidence, compatibility
history, and independent acceptance remain open, so no production-readiness
row closes. Local Linux correlation now
passes: the typed direct-exact route preserves verification and policy-refusal
semantics under a 30-second deadline, 64 KiB output cap, 16 MiB file cap,
64 MiB address-space ceiling, and process-group containment. The six-job
release-build acceptance pipeline also passes with every governed verification
under that address-space ceiling. Hosted Linux CI reproduces the governed
pipeline. Address space is not peak RSS, and this evidence is not independent
acceptance.

The governed proof-carrying MTBDD portfolio now binds caller-selected proof and
composition limits before proof checking or semantic replay. Its static route
uses proof-carrying MTBDD only after structural admission and exact direct replay
only for the three versioned structural rejection classes. Proof generation,
encoding, checking, malformed evidence, and query drift fail closed rather than
triggering fallback. Rust, file CLI, and typed bounded-process interfaces cover
both routes and seven typed resource refusals. A deterministic six-job pipeline
preserves 3 SAFE and 5 UNSAFE results across the public washing-controller batch
and a boundary fallback control, with four negative controls carrying no answer.
The acceptance CSV and both canonical artifact SHA-256 fingerprints are
byte-identical on macOS and an offline Linux container under a 64 MiB
address-space ceiling. This closes the local public-workflow, process-integration,
and compatibility-baseline mechanism gates. Exact-head hosted reproduction,
compatibility through a subsequent tagged release, independently sourced plant
validity, general partner source-to-model attestation, and independent suitability
assessment remain open. The pinned public controller and physical plant now
have deterministic exact-revision Yosys regeneration with retained source,
recipe, model, and regenerated-model SHA-256 evidence. This closes their local
source-to-model provenance mechanism gap, but does not independently prove Yosys
correct or cover external designs. Attested portfolio verification additionally
binds provenance to the exact source and model snapshot loaded for the query and
rejects post-snapshot replacement. No production-readiness row closes yet.

The changing-plant experiment now separates controller proof evidence from
content-addressed plant results and supports process-local proof admission.
Every ordered obligation field is caller-bound, and stale evidence, reordered
or duplicated obligations, mutation, and truncation fail closed. The local
five-plant fixture shows a 49.75% marginal byte reduction against the faithful
composed-witness route, but an 8.71 times larger initial package and a 58-change
byte break-even. Stable file and typed bounded-process interfaces plus hosted
cross-platform artifact reproduction are implemented. Compatibility through a
later tag and external acceptance remain open. A three-trial Darwin arm64
baseline now separates controller production, both replaceable plant-result
producers, and governed verification, retaining peak RSS, wall time,
deterministic evidence bytes, and exact answer agreement. Ordinary Linux CI
validates the same measurement schema and complete operation set without
comparing host-dependent values for equality. Hosted Linux run
29776279270 passes the real governed CLI and typed-client integration, including
every tight policy boundary and hostile response controls. Two one-property
public washing-controller manifests now seed compatibility history with exact
manifest, controller-evidence, and plant-result fingerprints. This adds a
candidate product mechanism and closes no production-readiness row.

The split Rust API now preflights controller artifact bytes and embedded UNSAT
proof bytes before admission, then independently preflights plant artifact
bytes, complete ordered obligations, horizon, product states, and conservative
transition evaluations before replay. Exact boundaries pass and tighter limits
fail closed. This closes the in-process resource-governance mechanism only.
The versioned file CLI now creates deterministic split artifacts and admits one
controller proof for multiple manifest/result pairs in a single process. It
reports per-batch verification time and one aggregate while failing closed on
controller, boundary, obligation, integrity, and argument drift. The typed Rust
client now applies bounded shell-free execution, strictly parses the discovered
contract and result rows, reconciles all batch totals with checked arithmetic,
and reports stable per-invocation metrics. A separate governed CLI and typed
client now enforce caller-selected controller, proof, batch, per-batch, and
complete-set limits before semantic replay. The two-pass verifier binds
manifest semantics, source/model snapshots, result digests, and resource
assessments across preflight and replay, emits no verified row on refusal, and
maps eleven stable refusal reasons to exit code 3 without a logical answer.
Retained self-service acceptance now aggregates one verified two-batch job,
three typed refusals, two invalid controls, structural resource totals, and
exact compatibility fingerprints while omitting non-reproducible timings.
Compatibility through a later tag remains open. Hosted Linux run 29777543062
reproduces the retained public acceptance CSV on exact commit `e74828f`.
Hosted amd64 run 29773273695 now reproduces the split controller and replacement
artifact hashes, structural byte accounting, independent composed-witness
hashes, and all logical answers from arm64. This closes the mechanism's second
architecture reproduction gate. Hosted run 29778509796 independently measures
the complete process-resource lifecycle on Linux x86_64 with identical evidence
bytes and answers. Compatibility through a later tag and independent acceptance
remain open.

The additive governed split observability contract now reports four versioned
internal phase durations and eleven checked structural work counters after a
complete success. Its typed client reconciles those counters with the strict
base result, rejects hostile or overflowing summaries, and returns no partial
measurement on refusal. The additive cache path reports process-local semantic
replay lookups, hits, misses, and entries only after full two-pass integrity
preflight. Allocator peaks, CPU counters, and per-phase peak RSS remain outside
this contract.
After fixture setup, the retained acceptance adds six observed-contract
invocations: discovery, three successful verifications covering four retained
batches, one duplicate-batch cache probe, and one fail-closed resource refusal.
Its canonical CSV aggregates deterministic work and evidence totals while
excluding host-dependent phase timings and the separate cache probe.
The workflow now exercises allocation observation on all three successes,
requires positive allocation calls and bytes, and still requires no stdout on
refusal. Allocation values remain live checks rather than portable retained
numbers.
Hosted run 29781337392 passes the underlying observed CLI and typed contract on
exact commit `6393ccf`, including all portable API, public RTL, hostile-input,
dependency, and reproducible-package companion jobs. Hosted run 29782315590
reproduces the retained multi-job CSV on exact commit `7b9e024`, with every
companion job green. Hosted run 29783651272 validates allocation observation on
exact commit `2c68e5e`, with every required job green.
Hosted run 29784705375 validates the cache contract on exact commit `eb8b6fd`,
including the portable API matrix, public RTL corpus, maintained Bitwuzla
baseline, dependency audit, and reproducible Linux bundle.

The [OpenTitan AON watchdog experiment](OPENTITAN_AON_WATCHDOG_EXPERIMENT_V1.md)
adds the first production-tagged public embedded core to the word-level path.
Pinned source and Yosys regeneration produce byte-identical models; retained
SAFE, UNSAFE, and billion-frame SAFE certificates verify independently; and
five hostile controls fail closed or route to exact fallback. The wrapper fixes
one watchdog configuration and supplies reduced register types, so it does not
close the real-product row or substitute for independent operator acceptance.
Hosted run 29787171907 reproduces this complete path on exact commit `6f0c4d4`,
including pinned Linux Yosys, maintained Bitwuzla, official BTOR2Tools, the
three-platform Rust API matrix, dependency audit, and reproducible packaging.

The predicate-set v2 follow-up keeps the additive Rust API and two self-service
CLI commands while sharing one exact recurrence across joint SAFE, mixed, and
joint UNSAFE batches. Each UNSAFE member carries an exact earliest frame and a
compact source-reconstructed witness kind. Unsupported batches preserve the
complete query through exact fallback, and any member failure returns no
partial batch. The pinned OpenTitan cases retain four v2 artifacts, verify three
v1 compatibility artifacts, compare every available separate baseline, record
the billion-frame bounded-search refusal explicitly, and exercise nine hostile
controls. Local reproduction is complete. Hosted reproduction for the exact v2
commit passes in
[run 29791772775](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29791772775),
including pinned Linux Yosys, official BTOR2Tools, maintained Bitwuzla,
three-platform downstream APIs, RustSec audit, the full retained workflow, and
reproducible packaging. A frozen first-release compatibility fingerprint,
external operator acceptance, and expert novelty review remain open.

The OpenTitan dual-timer probe expands the same pinned core from one live
timer path to wake-up and watchdog operation together. It adds exact parser
support for Yosys reduction-or and freezes a post-reset three-state model with
bad frames 5, 7, and 9. Predicate-set v3 now reconstructs the prescaler
invariant and both timer recurrences, then emits deterministic artifacts for
horizons 4, 5, 7, 9, and one billion. Local regeneration, exact frames, retained
v1/v2 compatibility, ten hostile controls, and every h9 truncation pass. The
separate baseline is explicitly unavailable, not counted as a win. Pinned
official BTOR2Tools locally parses the model and replays isolated witnesses at
all three exact bad boundaries, and maintained SMT agrees at all six boundary
queries. The local identical-scope AIGER control now agrees on all twelve
property answers. Certifaiger plus `lrat_isa` independently accepts six SAFE
certificates, `aigsim` replays six UNSAFE traces, and the two applicable SAFE
sets compose and verify. Model and evidence regeneration is byte-deterministic.
Hosted reproduction of this new baseline, Linux resource enforcement, targeted
mutation controls, and independent review remain open, so this does not yet
close a production-readiness row.

## Post-production-release deliverables

- Create a visually polished, accessible SVG architecture diagram after the
  first production release passes every applicable gate. It must show GCC's
  platform boundary, the CQ-SAT engine, source-to-model attestation, governed
  verification and exact fallback, proof artifacts, independent checking, and
  embedded firmware and RTL integration paths. Keep the source editable and
  publish an optimised web-ready SVG as the canonical asset embedded in both
  the project README and guardedcontinuation.org. Include accessible metadata,
  responsive sizing, and a lightweight fallback for non-SVG contexts.

## Rules

- No timing-based per-formula calibration.
- A specialised-backend error is never silently converted into a positive or
  negative verification answer.
- Fallback must preserve the original query and environmental assumptions.
- Performance evidence must report all negative rows and setup costs.
- “Production grade” remains prohibited wording while any required gate is
  open.
