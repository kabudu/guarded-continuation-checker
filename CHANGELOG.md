# Changelog

## Unreleased

- Cancel superseded CI runs for the same pull request or manually dispatched
  ref. Rapid experiment commits no longer leave obsolete full test and public
  RTL corpus jobs consuming hosted runner time.
- Predeclare a controlled OpenTitan PWM maintained-tool baseline that replaces
  40 isolated Docker launches with single-container sequential and fixed
  four-way parallel orchestration while preserving independent evidence
  checking, exact cohort hashes, and a threshold-free reporting rule.
- Qualify all ten predeclared single-container trials without selective reruns.
  Sequential and four-way parallel source-through-producer medians are 0.89
  and 0.82 seconds versus GCC's matched 0.09 seconds, reducing the comparison
  from 53.78 times against isolated containers to about 9.89 and 9.11 times.
  Retain the identical model and evidence hashes and disclose that most of the
  earlier ratio was container-launch overhead.

## 0.30.0 - 2026-07-22

- Predeclare revision impact certificate v1 for selective firmware regression.
  Freeze exact source-bound reuse, exhaustive bounded counterfactuals, complete
  inclusion-minimal invalidating sets, both-answer replay, resource governance,
  hostile-input rejection, exact fallback, maintained full-rebuild comparison,
  public semantic-revision evidence, self-service integration and three-platform
  identity gates. Record incremental IC3, precision reuse, mutation-impact
  propagation and hierarchical RTL lemma reuse as disconfirming prior art, so
  no broad incremental-verification novelty claim is permitted.
- Implement the first public revision-impact certificate core. Canonical
  bounded atoms, dependency edges, query support, complete counterfactual
  observations and inclusion-minimal invalidating sets round-trip through a
  strict binary format. Verification rederives every minimal set and requires
  an independent semantic evaluator for every table entry. Five API tests cover
  deterministic round trips, semantic disagreement, graph and support errors,
  truncation, mutation, trailing data and missing or extra minimal sets. This
  closes the certificate-mechanics slice only; exact revision-local integration
  remains open.
- Bind every impact observation to the SHA-256 of canonical revision-local
  evidence. Add an exact two-component adapter that enumerates every old/new
  source combination, produces its bounded answer, independently decodes and
  verifies each semantic artifact, and rejects query, boundary, source,
  evidence, digest or result drift. A both-answer mechanism test preserves an
  UNSAFE-to-SAFE semantic revision and an unchanged SAFE query. Public-product
  integration and the complete four-transition answer cohort remain open.
- Add a canonical aggregate impact bundle and symmetric production,
  verification, encode and decode policies for total input bytes, per-scenario
  evidence, total bundle bytes, counterfactual combinations and query count.
  Seven focused tests now include every aggregate policy dimension and every
  bundle truncation. Production, encoding, decoding and verification enforce
  the same caller-selected dimensions. Resource refusal occurs before retaining
  or decoding an over-limit artifact.
- Add the first self-service revision-impact file CLI. Strict old/new component
  and interface inputs plus a canonical bounded-query manifest produce or
  independently verify the aggregate certificate. The workflow uses bounded
  non-symlink reads, exact query and source binding, atomic no-clobber output,
  and fail-closed rejection of manifest drift, certificate mutation, and
  noncanonical text. This closes the file-CLI portion of the integration gate;
  the typed process client, capabilities, baselines, public cohort, and
  portability evidence remain open.
- Add a strict machine-readable capability contract and the public
  `RevisionImpactTool` process client. Discovery pins the v1 semantic,
  no-routing, no-fallback, fail-closed contract and every supported resource
  ceiling. Certify and verify calls are shell-free, deadline/output/file
  bounded, validate output-node lists before spawning, parse one exact summary
  schema, and expose success and failure invocation metrics. The client default
  file limit now matches the advertised 64 MiB bundle ceiling.
- Add deterministic verifier-work observability to the exact aggregate path.
  Independent checking now reports parsed evidence bytes, semantic replays,
  component validations, composed pair checks, final transition checks, and
  result comparisons with checked arithmetic and completeness invariants. The
  CLI and typed process client expose the same `verification-v1` work schema;
  these counters describe logical work and never substitute wall time. File-CLI
  tests admit the inclusive 32-query boundary and refuse query 33, oversized
  manifests, noncanonical text, and manifest symlinks without partial output.
- Add strict per-query `transition-v1` output and typed parsing for old and new
  SAFE or UNSAFE results. Retain the first public OpenTitan `prim_count`
  revision-impact cohort spanning unchanged UNSAFE, UNSAFE-to-SAFE, unchanged
  SAFE, and SAFE-to-UNSAFE in one exact bundle. Two clean productions emit the
  same 56,632,691-byte certificate and SHA-256, and independent verification
  repeats all eight semantic results and deterministic work counters. The
  portable-host gate remains open.
- Qualify the identical four-query maintained baseline. Pinned Yosys plus rIC3
  and Certifaiger agree on four SAFE and four UNSAFE scenario answers and
  independently validate every witness or trace. Their complete 2,658-byte
  model-plus-evidence package is about 21,306.5 times smaller than GCC's
  56,632,691-byte aggregate. GCC source-through-production and checker peak RSS
  are about 17.7 and 57.3 times larger. GCC also takes 3.03 seconds from
  synthesis through production versus 1.81 seconds for the maintained route.
  This closes the local semantic, transfer, evidence-checking, peak-memory, and
  total producer-time slices with a strong negative result. The apparent GCC
  checker-time advantage is not claimed because the maintained checker uses
  eight isolated Docker launches. No novelty claim is made.
- Freeze an 888-byte revision-impact certificate fixture at SHA-256
  `63f65c7ee9c8a296af0f2dace3cea9f129f159bc70f2fafb33633f70724b12f0`.
  Hosted Actions run 29905014426 reproduces the exact bytes, decoding, and all
  16 independently replayed observations on Linux, macOS, and Windows. This
  closes the certificate-format portability gate; larger public-subsystem and
  independent-review gates remain open.
- Predeclare the larger public two-atom revision cohort against OpenTitan PWM
  commit `86db2898288664d8d5e8fc635b48951ef63e3439`. Freeze the parent/child
  source digests, connected core-clear and channel-output atom boundary, five
  query classes including a combination-only regression, provenance,
  deterministic production, independent replay, hostile drift, maintained
  proof-producing baseline, resource, and hosted release-build gates before
  extracting or measuring the fixture.
- Implement the OpenTitan PWM two-atom cohort from specialised, source-digested
  SystemVerilog and reproducible pinned-Yosys BTOR2. Add an exact public API
  distinction between minimal evidence-invalidating sets and minimal semantic
  answer-changing sets, plus machine-readable CLI v2 output and strict typed
  process parsing. The four-combination matrix proves separate `{core}` and
  `{channel}` fixes and a joint `{core, channel}` regression while preserving
  unchanged SAFE and UNSAFE controls. Two clean local productions emit the
  same 128,768-byte aggregate; maintained and hosted gates remain open.
- Qualify the matching OpenTitan PWM maintained baseline across all four source
  combinations and five properties. Pinned Yosys, rIC3 and Certifaiger agree
  with GCC on 9 SAFE and 11 UNSAFE observations and independently validate all
  20 artifacts. The maintained model-plus-evidence package is 15,479 bytes,
  making GCC's current aggregate about 8.32 times larger. Retain this negative
  result while the matched GCC resource, hostile-drift, exact-patch and hosted
  gates remain open.
- Add five matched arm64 trials for the OpenTitan PWM cohort. GCC's median
  source-through-answer time is 0.09 seconds versus 4.84 seconds for 20
  isolated maintained jobs, about 53.78 times faster, with about 20.0% lower
  producer-path peak RSS. GCC's artifact remains about 8.32 times larger. Treat
  this as a scoped shared-model and orchestration advantage, not a universal
  solver-performance claim.
- Retain the exact 3,810-byte upstream OpenTitan PWM commit patch and bind its
  authoritative SHA-256 into both source-generation routes. Add a public-cohort
  hostile matrix proving fail-closed rejection of source, interface, query,
  revision-direction, atom-order, evidence, digest, result and minimal-set
  drift.
- Add a dedicated hosted Linux release-build gate for the public OpenTitan PWM
  cohort. Actions run 29910725650 reproduces the exact 128,768-byte bundle and
  frozen SHA-256, all 20 observations, three minimal semantic change sets and
  all nine hostile rejections. This closes every predeclared cohort gate, not
  the broader production or novelty programme.
- Predeclare QatQ transport qualification v1 as an additive research-only
  experiment. Freeze the exact QatQ boundary and require a GCC-owned
  length, digest, codec and resource envelope, streaming exact recovery,
  hostile-input rejection, cross-platform identity, realistic resource
  measurements, semantic replay and retained negative compression rows before
  considering integration. QatQ remains outside `firmware-rtl-v1`.
- Implement the optional `research-qatq-transport` Rust API against exact QatQ.
  Add a canonical length, codec, parameter, canonical-digest and
  encoded-digest envelope; checked resource policy; pre-allocation per-chunk
  framing limits; independent QatQ checksum validation; chunked exact recovery;
  and atomic create-new file publication. Eight boundary and hostile tests pass.
  On the verified OpenTitan revision batch the 82,428-byte envelope is 29.41%
  smaller than zstd level 22 long-window, with median 133.008 ms encode, 39.104
  ms decode and 66,240,512-byte process peak RSS on arm64. Retain the negative
  maintained-proof-package row, where QatQ is 73.23% larger than zstd. Hosted
  run 29893368169 reproduces the frozen envelope on Linux, macOS and Windows;
  Linux records 456.823 ms median encode, 79.591 ms median decode and
  68,968,448-byte peak RSS.
- Upstream and adopt QatQ 0.1.4's exact-byte container and bounded byte-chunk
  visitor APIs. Remove GCC's private byte-to-`u32` packing, QATC/QATQ framing
  parser and public floating-point-labelled mapping without changing a frozen
  envelope byte. Preserve chunk-bounded recovery, callback-before-validation
  exclusion, resource policy, hostile rejection and the prior exact hash.
  Compatibility history and independent review remain open promotion gates.

## 0.29.0 - 2026-07-22

- Bound the crates.io source payload to the executable and library sources,
  licence and user-facing READMEs. Add a 64-file and 768 KiB compressed package
  gate that excludes CI, corpora, retained results, research examples, scripts
  and tests from the installed product payload. Permit Cargo's platform-specific
  empty auto-target directories while continuing to reject files and symlinks
  below them, and root-anchor every include pattern so nested research READMEs
  cannot enter through basename matching.

- Freeze production support profile v1 around firmware CLI contract v2 and RTL
  artifact schema v4. Add a build feature and machine-readable capability that
  make the profiled binary reject every research command before dispatch. Add
  an executable boundary gate covering all eight supported commands, six named
  research surfaces, the legacy experiment path, empty invocation and invalid
  capability arguments. Research builds and later additive releases remain
  available without expanding the first production compatibility promise.
  Add a distinct deterministic Linux production-candidate archive that builds
  the profiled binary, ships only supported contracts, binds the profile into
  capabilities, build information and provenance, verifies without execution,
  and is rebuilt twice in required CI. The existing broad evaluation archive
  remains reproducible under its original profile.
- Add a QatQ exact compression probe over the 14,164,144-byte OpenTitan
  revision batch. The best exact f32-word mapping produces 76,385 bytes and
  restores the canonical batch bit-for-bit. It is 34.5845% smaller than the
  strongest measured zstd configuration, but remains 8.59 times larger than
  the maintained model-plus-evidence package. Keep QatQ outside the first
  production support profile pending an opaque-word API, resource evidence,
  portability and hostile-container testing.
- Add revision batch certificate v1, a canonical content-addressed format that
  stores each validated local relation once and binds heterogeneous queries to
  shared sections. Add typed production, canonical encoding and decoding,
  independent source replay, exact standalone extraction, strict limits and a
  hostile mutation matrix. On 16 OpenTitan queries the batch removes
  99,100,424 duplicated bytes, preserves all answers and extracts every
  standalone certificate byte-identically. At 14,164,144 bytes it remains
  about 1,593 times larger than the qualified maintained model-plus-evidence
  route, so this closes a container defect without establishing novelty.
- Add an eight-property repeated-query workload over the authentic OpenTitan
  `prim_count` semantic revision. Reusing two validated relations preserves
  byte-identical artifacts and all 16 answers while reducing local candidate
  work by 87.5006%; median internal production and verification ratios are
  0.213828 and 0.071409. An equivalent qualified Yosys, rIC3, and Certifaiger
  baseline agrees on every answer using 8,892 total model and evidence bytes,
  versus 113,264,568 embedded GCC certificate bytes. This falsifies a broad
  artifact advantage and motivates a content-addressed shared-section batch
  certificate instead of another standalone-certificate optimisation.
- Add a typed repeated-query API that accepts two previously validated local
  relation artifacts, produces no new local sections, reuses both sections
  byte-for-byte, and regenerates only the query-specific bounded answer. Its
  verifier performs no local decoding or semantic replay after validation,
  recomposes the validated relations, and checks the final proof. Add exact
  from-scratch artifact identity, SAFE result, work accounting, and right-side
  substitution rejection tests.
- Add the first authentic stable-interface revision cohort with a reachable
  semantic change. OpenTitan `prim_count` moves from SAFE to UNSAFE at reset
  across commit `369cffc8`; GCC reuses and reverifies the unchanged environment
  relation while recomputing the changed counter relation. Add deterministic
  model and certificate manifests, a separate Yosys plus Z3 oracle, strict
  source provenance, and hosted reproduction. Add a pinned Slang-enabled Yosys
  gate that compiles both untouched upstream revisions and proves each selected
  specialisation sequentially equivalent. This gate exposed and corrected the
  initial fixture's collapsed comparison-valid enum. A maintained
  equivalent-scope rIC3 and Certifaiger baseline proves the old SAFE witness and
  new UNSAFE trace, rejects both cross-revision evidence swaps, and regenerates
  only a 13-byte trace. This qualifies the functional distinction but
  falsifies a GCC certificate-byte advantage on the tiny cohort.
- Add bounded search certificate v5 for one through eight word-valued semantic
  inputs with at most eight total bits. Bind source input widths, reconstruct
  fields in input-node and least-significant-bit order, preserve optional exact
  constraints and assumption dead ends, and charge work against the complete
  packed valuation space. Add an independent exhaustive oracle across widths
  two through eight, downstream API and no-clobber CLI coverage, targeted
  hostile controls, and deterministic resource refusal. Retain v1 through v4
  selection and encoding. Add a pinned Caliptra watchdog workflow with a live
  two-bit timeout field, exact nonzero constraint, SAFE and UNSAFE results,
  deterministic evidence, seven hostile controls, and maintained Yosys plus Z3
  agreement. Hosted amd64 run 29874337371 reproduces all evidence and passes the
  complete platform and release-build matrix.
- Add bounded search certificate v4 for exact BTOR2 environment constraints.
  Bind ordered input and constraint nodes, quantify only admissible all-frame
  valuations, preserve assumption dead ends, and charge governance against the
  full valuation space. Recheck every UNSAFE transition and terminal valuation
  against the bound all-frame constraints, with forbidden-transition and
  forbidden-terminal regression cases. Add a constrained public Roa Logic PLIC
  workflow with deterministic source and evidence, hostile constraint controls, an
  independent exhaustive oracle, and maintained Yosys plus Z3 agreement through
  horizon 16. Retain v1, Caliptra v2, and PLIC v3 results byte-for-byte. This is
  product-workflow evidence, not an algorithmic novelty claim.
- Add bounded search certificate v3 for two through eight independent one-bit
  semantic inputs. Bind ordered input nodes, complete packed transition
  valuations, and a distinct UNSAFE terminal valuation; enumerate every
  valuation when independently reconstructing SAFE layers; account for the
  valuation count before search; and reject reorder, high-bit, downgrade,
  truncation, and resource attacks. Preserve retained v1 and v2 evidence
  byte-for-byte. Add a revision-pinned unmodified Roa Logic PLIC gateway whose
  five-input wrapper properties move from governed refusal to exact SAFE
  answers through horizon 16, agree with maintained Yosys plus Z3, and retain a
  horizon-64 node-step refusal. This is public-RTL interoperability and exact
  fallback evidence, not an algorithmic novelty claim.
- Add bounded search certificate v2 for bad properties that depend on the
  current one-bit semantic input. Preserve transition inputs separately from
  the terminal-frame input, check both terminal values over every complete SAFE
  layer, reject cross-version reinterpretation and downgrade, and continue to
  produce retained byte-identical v1 evidence for state-only properties. Turn
  the pinned Caliptra word-level horizons 2, 3, and 5 from governed refusals
  into independently verified exact answers while retaining the billion-frame
  resource refusal. This closes a Yosys asynchronous-reset interoperability
  gap; it is conventional explicit search, not an algorithmic novelty claim.
- Add the pinned OpenTitan AON dual-timer structural probe for invariant-chained
  predicate composition. Enable wake-up and watchdog paths together, retain a
  deterministic three-state BTOR2 model, implement standard reduction-or word
  semantics in the strict parser, and validate wake-up, bark, and bite boundary
  frames through the public semantic API. Make the post-reset zero-initial state
  assumption explicit in the Yosys builder and provenance. Predeclare exact
  multi-recurrence acceptance criteria and retain predicate-set v2's complete-
  query refusal as the negative control. This identifies the next mechanism; it
  does not yet implement invariant chaining or establish novelty.
- Add an exact BTOR2 predicate-set portfolio with canonical v1 and v2
  source-bound certificates. Version 2 shares one recurrence claim across 2 to
  64 ordered SAFE and UNSAFE predicates, carries exact earliest bad frames, and
  reconstructs compact `advance_prefix` witnesses instead of embedding separate
  traces. Reproduce routing during independent verification, reject omitted,
  reordered, substituted, or downgraded queries, and preserve unsupported
  complete queries through the existing exact per-property portfolio without
  partial answers. Continue to decode and verify retained v1 artifacts under
  their original selection rules. Add stable Rust and no-clobber CLI surfaces,
  bounded canonical decoding, mutation tests, and a pinned OpenTitan
  bark-plus-bite workflow. The retained public RTL cases reduce certificate
  bytes by 41.8% for joint SAFE, 30.9% for mixed UNSAFE and SAFE, and 41.1% for
  the billion-frame scale case. A 384-byte shared artifact also gives exact bad
  frames 5 and 9 at a billion-frame horizon where the separate bounded search
  baseline refuses both queries. Add official BTOR2Tools parsing, maintained
  Bitwuzla endpoints, deterministic Yosys regeneration, nine hostile controls,
  and explicit prior-art boundaries. This is a narrow candidate contribution,
  not an established novelty or production claim.
- Accept standard Yosys BTOR2 observation statements and optional node symbols,
  while exposing only inputs that reach transition, constraint, or bad-property
  semantics. Admit exact Boolean identity wrappers in the independently checked
  word-region route and reject an XOR near-neighbour to exact fallback. Add a
  source-bound, production-tagged OpenTitan AON watchdog target with pinned
  Yosys regeneration, deterministic SAFE and UNSAFE certificates, a
  billion-frame compact proof, BTOR2Tools and Bitwuzla controls, public Rust API
  coverage, five hostile controls, and retained self-service acceptance. This
  closes a narrow public-RTL integration mechanism, not product validity,
  production readiness, or scholarly novelty.
- Add an additive governed split observability contract without changing the
  existing strict resource capability or result rows. Report four versioned
  phase durations and eleven checked structural work counters only after a
  complete success. Add a bounded shell-free Rust client that strictly parses
  and reconciles every counter, rejects hostile or overflowing helper output,
  and preserves typed refusals plus whole-process metrics without returning a
  partial observed result. After fixture setup, retain five observed-contract
  invocations across discovery, three successful requests, four verified
  batches, and a refusal that emits no partial metrics. Reproduce their
  deterministic structural aggregate in ordinary Linux CI while excluding
  host-dependent timings.
  Add a separate allocation-observability v1 discovery, command, strict final
  row, and typed Rust client without changing either existing observability v1
  response. Count successful system-allocation, deallocation, and reallocation
  events plus requested bytes across the policy-through-replay scope. Use an
  opt-in concurrency barrier, fail closed on overflow or hostile summaries, and
  emit no partial metrics on refusal. Exercise positive allocation evidence in
  every successful retained observed request while keeping allocator-dependent
  totals out of portable CSV evidence.
  Add an integrity-preserving process-local semantic replay cache behind a new
  additive discovery, command, strict result row, and typed Rust client. Require
  complete manifest, source/model snapshot, result-digest, and resource
  revalidation before lookup. Reconcile lookup, hit, miss, and entry counters,
  reject hostile summaries, retain old commands on their uncached paths, and
  exercise a deterministic duplicate-batch hit without changing the portable
  structural CSV.
- Add the first Rust API slice for governed proof-carrying controller MTBDD
  verification. Bound the complete artifact, equivalence artifact, embedded
  UNSAT proof, members, horizons, product states, and transition evaluations
  before proof checking or semantic replay. Bind every ordered member and
  preserve the existing independently checked proof path with zero exhaustive
  controller assignments. Add canonical policy and capability contracts, a
  governed verification command, seven stable refusal reasons, exit code 3
  without a logical answer, strict response parsing, and a typed shell-free
  process client. Add a canonical proof/direct portfolio whose fallback is
  limited to the three structural MTBDD rejection classes. Reproduce the route
  during verification, reject forced downgrade, preserve both answer classes,
  and reject every retained outer-artifact mutation. Portfolio typed-process,
  public-product, and compatibility gates remain open. Add versioned portfolio
  capability, certification, verification, and governed-verification file
  commands. The admitted proof route reports its structural decision, preserves
  both answer classes and zero exhaustive assignments, and refuses a tight
  proof budget before proof checking. Add a strict typed portfolio process
  client and direct-route CLI acceptance with proof limits deliberately ignored
  only when no proof is present. Add deterministic macOS/Linux acceptance for the six-property public
  washing-controller proof route, exact fallback, two typed refusals, malformed
  policy, and corrupt evidence. Hosted reproduction and compatibility remain
  open. Freeze SHA-256 fingerprints for both routed v1 artifacts so a subsequent
  tagged release can prove byte compatibility.
  Bind standalone portfolio resource assessment to caller-supplied relevant
  inputs, observed outputs, and member wiring before workload calculation.
  Add canonical source-to-model provenance for the public controller and plant,
  with exact-revision isolated Yosys regeneration, deterministic SHA-256
  evidence, CI reproduction, and hostile manifest/model controls. Add a public
  canonical attestation verifier and an attested governed-portfolio command
  that binds the exact controller and distinct plant source/model subjects
  before returning any answer. Expose the same fail-closed path through the
  typed Rust client and exercise it in public acceptance. Bind the attestation
  pass to source and model digests captured during query loading so concurrent
  post-snapshot file replacement cannot substitute the verified semantics.
  Make exact regeneration portable across compilers by checking the pinned
  binary revision separately and using Yosys `--no-version` for deterministic
  AIGER footers.
  Bind the external production gate to the exact evidence-register digest and
  require an OpenSSH signature from the attested independent reviewer under a
  fixed namespace and caller-controlled allowed-signers policy. Add hostile
  register substitution, attestation tampering, and untrusted-key controls.
  Add bounded prior-art audit v1. Record the disconfirming FMCAD 2023 and FM
  2026 compositional certificate results and relevant predicate-abstraction
  patent records. Narrow the research target to reusable controller evidence
  across independently changing plant contracts, with a required direct
  composed-witness baseline before any novelty claim. Predeclare that baseline
  against the FM 2026 construction, including four changing-plant fixtures,
  exact incremental-rebuild accounting, independent Certifaiger validation,
  hostile controls, and explicit falsification criteria. Record that no public
  `aigmerge` implementation was present in the inspected upstream branches, so
  ordinary per-property Certifaiger checking cannot stand in for composition.
  Freeze the first four-plant comparison family with nominal, sensor-stuck,
  actuator-delay, and persistent-disturbance semantics. Reproduce each model
  byte-for-byte from source with pinned Yosys, independently replay all 24
  horizon-32 answers and shortest traces, preserve two SAFE properties per
  plant, and add source, model, and member-substitution hostile controls.
  Add a safety-only FM 2026 Theorem 1 baseline implementation with a versioned
  Rust API and CLI. Coalesce shared model mappings, preserve private variables,
  union reset and transition functions, conjoin safety and constraints, and
  deterministically hash-cons gates. Fail closed on bounded parsing, mapping
  mismatches, liveness, comment mappings, truncation, symlinks, and output
  collisions. Validate an immutable upstream witness self-composition with
  qualified Certifaiger 10.2.0 and formally verified `lrat_isa`; retain its
  exact hashes and add hosted amd64 reproduction. Distinct-property composition
  and the changing-plant comparison remain open, so this establishes baseline
  fidelity only and no novelty result.
  Extend the bounded AIGER exporter with deterministic multi-property bad
  sections and freeze paired single-property and shared two-property models for
  all four changing plants. Generate eight separate SAFE witnesses with pinned
  rIC3, independently validate each, compose each plant's two properties with
  the FM 2026 baseline, and validate all four composed witnesses with
  Certifaiger plus `lrat_isa`. The composed total is 9,665 bytes versus 19,164
  bytes separately, a 49.57% reduction and a negative result for any broad GCC
  evidence-size claim. Incremental plant-replacement cost remains the active
  candidate distinction. Add canonical split artifacts for content-addressed
  controller MTBDD equivalence evidence and separately replaceable plant
  results. A typed admitted-controller capability checks the controller proof
  once per process and permits later plant batches to reuse that verified state;
  the stateless convenience path remains fail closed. Bind plant results to the
  exact controller-evidence SHA-256 and reject stale evidence, source drift,
  cross-plant substitution, mutation, and truncation. Real-family marginal-cost
  measurement now replaces the third member of a four-plant package with a
  fifth pinned-Yosys-attested actuator transport-lag revision. Bind the complete
  ordered property, wiring, state, and horizon obligation and reject missing,
  duplicated, reordered, mutated, truncated, or stale evidence. GCC transfers
  4,160 marginal bytes versus 8,278 for the independently accepted FM 2026-style
  composed-witness path, a 49.75% reduction, but its initial package is 8.71
  times larger and breaks even only after 58 observed replacements. Hosted
  replacement reproduction, controlled resource accounting, file and process
  APIs, and resource-envelope integration remain open. Add the replacement
  baseline and split-evidence measurement to the exact hosted amd64 evidence
  workflow; a successful exact-head run is still required before treating the
  cross-platform gate as closed. Add two-stage Rust resource governance for the
  split route. Controller evidence and its embedded UNSAT proof are bounded
  before proof checking and admission; replaceable plant artifacts, ordered
  obligations, horizons, product states, and conservative transition work are
  bounded before semantic replay. Exact inclusive boundaries pass and each
  tighter byte, proof, horizon, state, or transition limit fails closed.
  Add split-evidence CLI v1 with strict capability discovery, deterministic
  controller-evidence and plant-result producers, exclusive output creation,
  and one multi-batch verifier process. The verifier admits the controller proof
  exactly once, rejects controller or boundary drift across batches, replays
  every complete ordered obligation, emits per-batch timings and an aggregate,
  and returns no aggregate on malformed, stale, mutated, or incomplete input.
  Hosted amd64 run 29773273695 reproduces the arm64 split controller and
  replacement artifact SHA-256 values, structural byte accounting, ten SAFE
  results, unchanged-member identity, and all three independently checked
  replacement-witness hashes. Timing observations differ by host and are not
  represented as reproducible measurements. Add the typed shell-free
  `ControllerSplitEvidenceTool` with strict contract discovery, bounded process
  execution, deterministic artifact-production summaries, one-admission
  multi-batch verification, stable invocation metrics, checked aggregation,
  and complete cross-row reconciliation. Reject empty or excessive sets before
  invocation and reject malformed, inconsistent, overflowing, or changed
  helper responses without returning a verified summary. Add canonical
  caller-selected split resource policy v1, discovery, governed multi-batch CLI,
  and typed `ControllerSplitResourceTool`. Bound controller and proof evidence,
  per-batch composition work, batch count, and complete-set bytes, members, and
  conservative transitions. Preflight the entire request before replay, bind
  manifest semantics and source/model/result snapshots across both passes,
  buffer output until every batch succeeds, and return no logical answer or
  verified row for eleven typed exit-code-3 refusal classes. Add deterministic
  self-service acceptance over two independent public washing-controller plant
  batches, three resource refusals, malformed-policy and corrupt-evidence
  controls, stable structural aggregation, and frozen SHA-256 fingerprints for
  both manifests plus controller and plant artifacts. Reproduce the retained
  CSV in ordinary Linux CI without treating timing as portable evidence. Hosted
  run 29776279270 passes the real governed CLI and typed-client integration at
  exact commit `1227d50`. Hosted Linux run 29777543062 reproduces the retained
  CSV on exact commit `e74828f` and passes every companion CI job.
  Add a cross-platform whole-process resource harness that separately measures
  controller certification, both independently replaceable plant-result
  producers, and one-admission governed verification. Retain three Darwin arm64
  trials with exact answer agreement, portable evidence bytes, wall time, and
  peak RSS. Require ordinary Linux CI to exercise the same complete operation
  set while treating timings and memory as architecture-labelled observations,
  not reproducible values or routing inputs. Hosted run 29778509796 passes every
  job at exact commit `1960cd3`; retain its three Linux x86_64 trials separately
  with identical evidence sizes and answers.
  Add an explicit compatibility and migration policy for the first production
  line, including strict contract-version semantics, a minimum support window,
  fail-closed unsupported-version behavior, immutable upgrade and rollback
  rules, and registry SemVer expectations. Promote the deterministic split-v1
  self-service fixture into an executable release-compatibility gate while
  retaining the honest distinction between a candidate baseline and later-tag
  compatibility history. Reconcile the readiness register with the already
  implemented operations runbook and executable Linux qualification evidence.
  Add bounded process-client metrics aggregation schema v1. Use checked totals,
  a one-million-job cap, canonical operation and failure distributions, and
  explicit process-group and memory-limit coverage without changing the
  existing per-invocation metrics schema. Exercise the public API against real
  split resource discovery, successful governed verification, and a typed
  resource refusal, retaining all three jobs in the aggregate. Internal cache,
  allocation, and phase-resource counters remain open.

- Add experimental controller/plant verification resource envelope v1 through
  the public Rust API. Preflight artifact bytes, members, horizons, product
  states, external-input branches, and direct-backend controller evaluations
  with checked conservative arithmetic before semantic replay. Preserve the
  existing exact MTBDD/direct portfolio, return the assessment separately from
  the verification answer, and fail closed at every caller-selected limit.
  Add a canonical bounded policy file, separate machine-readable capability and
  verification commands, hostile parser controls, and the typed shell-free
  `ControllerPlantResourceTool` with strict response parsing and invocation
  metrics. Distinguish policy refusal with exit code 3, five versioned refusal
  reasons, no logical answer, a typed `ResourceRefused` error, and the
  `resource_refusal` metrics class. Correlate the typed client with a 30-second
  deadline, 64 KiB output cap, 16 MiB file cap, 64 MiB Linux address-space
  ceiling, and process-group containment. Run every governed verification in
  the release-build acceptance pipeline under the same Linux address-space
  ceiling locally and on a hosted Linux runner. Independent
  constrained-workflow acceptance remains open. Add a reproducible six-job
  release-build acceptance pipeline with two exact verified batches, two valid
  policy refusals, malformed-policy and corrupt-evidence controls, and a
  byte-stable aggregate retaining every row. Preserve 3 SAFE and 5 UNSAFE
  answers and distinguish two verified, two refused, and two invalid jobs. Gate
  the retained result in Linux CI.

- Pinned and offline-qualified the Certifaiger 10.2.0, AIGER, CaDiCaL,
  `lrat_isa`, and runlim comparison stack on local arm64 Linux, including all
  upstream Certifaiger witness fixtures and the intentionally invalid control.
- Pinned and offline-qualified rIC3 1.5.2 with its recursive source and Cargo
  dependency graph, then independently replayed one SAFE certificate with
  Certifaiger and one UNSAFE trace with `aigsim`.
- Add bounded-equivalent AIGER export v1 for sampled controller and plant
  queries. Freeze six horizon-32 washing-controller models only after an
  independently implemented parser and reachable-state replay reproduced all
  answers and shortest bad frames. Validate all four UNSAFE traces with
  `aigsim` and both SAFE witnesses with Certifaiger, CaDiCaL, and `lrat_isa`.
- Retain the negative same-host Certifaiger-equivalent comparison: standard
  evidence is 48.97 times smaller, production is 6.57 times faster, checking
  is 1.63 times faster, and its tools are smaller. Retain the provisional GCC
  advantages of 45.8% lower producer space and 18.14 times lower verifier
  space. Reject seven hostile package controls. Record that plain IC3 and
  first-answer portfolio traces do not reliably preserve shortest bad frames;
  use one static BMC/IC3 minimality race for every external formula.
- Reproduce the Certifaiger-equivalent comparison on hosted Linux amd64 from
  clean pinned sources. Standard evidence remains 48.97 times smaller, is
  13.00 times faster to produce and 2.21 times faster to check. GCC retains a
  40.0% producer-space and 16.43-times verifier-space advantage. Retain exact
  per-property sizes and digests, all seven hostile-control results, tool and
  image provenance, and byte-identical external witnesses and GCC proof across
  arm64 and amd64. Add no automatic portfolio route from this narrow memory
  result.

- Add proof-carrying controller MTBDD CLI v1 and the typed
  `ControllerProofMtbddTool`. Keep compact discovery unchanged, reuse the
  canonical plant manifest, enforce create-new output, source and query binding,
  bounded UNSAT-miter verification, deterministic bytes, typed observations,
  and fail-closed mutation and drift handling. Discovery reports separate outer
  equivalence-artifact and embedded UNSAT-proof byte limits.
  Retain a nine-case public self-service acceptance fixture covering exact
  SAFE/UNSAFE answers, shortest bad frames, proof verification, manifest drift,
  artifact mutation, and output collision.
  Add a five-trial release-build whole-process baseline retaining exact answer
  agreement, 1.64x faster median creation, and 2.00x faster median verification
  at the existing 29.39x artifact-size cost. Retain the negative resource
  result: proof peak RSS is 1.29x higher for creation and 2.39x higher for
  verification on the measured arm64 host. Retain a GitHub-hosted Linux x86_64
  replication with exact agreement, 1.62x faster proof creation, 1.95x faster
  proof verification, nearly tied creation RSS, and 1.77x higher proof-verifier
  RSS.
  Add an identical-query maintained-tool baseline with exact agreement across
  five local trials. Retain the negative runtime result: SymbiYosys/Yosys/Z3 is
  1.33x faster than fresh proof verification and about 5.39x faster than initial
  creation plus verification. Retain the positive consumer tradeoff of 65.5%
  lower verifier peak RSS and portable evidence. Retain a hosted Linux
  reproduction showing the same runtime loss and 79.8% lower proof-verifier
  peak RSS.
  Predeclare the next equivalent-evidence comparison against the
  competition-standard Certifaiger and `aigsim` certificate path, including
  explicit semantic-equivalence, hostile-input, resource, and falsification
  gates.

- Add a source-bound controller MTBDD equivalence miter and bounded UNSAT proof,
  plus canonical `GCCMEP01` proof and `GCCMPF01` proof-carrying plant-batch
  artifacts. On the public 131,072-assignment controller, one retained arm64
  run checks the 242,496-byte proof in 0.82% of exhaustive replay time. Keep the
  byte tradeoff and established equivalence-checking prior art explicit; the
  proof path is not yet the portfolio default. The integrated six-property
  physical-plant path verifies in 49.7% of compact exhaustive time with exact
  agreement, while its artifact is 29.39 times larger. Report zero exhaustive
  assignments checked on this path so represented miter scope is not
  misreported as replay work.

- Add phase-level portfolio observability for model loading, artifact handling,
  semantic verification, publication, and complete elapsed time. Retain the
  finding that semantic replay, not source/model loading, dominates the public
  admitted batch and keep every timing field outside routing decisions.

- Make both washing-controller formal oracles portable across Yosys versions.
  Require byte-identical controller and plant regeneration only with the
  recorded Yosys 0.67 build; other hosts still verify pinned source and model
  digests plus formal properties and explicitly report source-to-model
  regeneration as skipped.

- Add a timing-free controller MTBDD plant portfolio through public Rust and
  self-service file interfaces. Select the
  reusable MTBDD artifact when admitted and preserve the identical ordered
  bounded query through independently replayed direct exact evidence only for
  explicit boundary, terminal, or node-limit rejection. Bind the selected
  backend, reason, boundary, member queries, sources, and payload; reject
  malformed models, semantic errors, query drift, mutation, and downgrade
  attempts. Add typed capability discovery, bounded subprocess execution,
  stable route and member results, and a six-case Linux acceptance gate.

- Add a portable three-way process-resource harness for controller MTBDD
  production, independent verification, and the maintained SymbiYosys oracle.
  Retain the negative 2.67-times verification-speed result and the positive
  85.2% peak-RSS reduction on one arm64 host without treating the different
  checked scopes as equivalent.

- Add experimental controller MTBDD plant CLI v1 with machine-readable
  discovery, a strict canonical source and query manifest, create-new batch
  production, independent file verification, exact per-member reports, hostile
  input regressions, and a six-member self-service acceptance harness.
  Add the typed, shell-free `ControllerMtbddTool` Rust client with strict
  capability and result parsing, bounded subprocess execution, ordered member
  results, aggregate consistency checks, and invocation metrics.

- Add a maintained single-session SymbiYosys, Yosys, and Z3 baseline for the
  six-property stateful plant. Reuse one compiled closed-loop model, reproduce
  all four shortest bad frames, prove the other two properties through frame
  32, retain machine-readable agreement, and gate the batch in CI.

- Expand the controller MTBDD static boundary from four to eight observed
  actions without relaxing assignment, node, terminal, or artifact limits. Add
  a seven-action fill-to-spin process whose 254-node, 189-terminal MTBDD checks
  three stateful safety properties through horizon 64 with exact direct-model
  agreement.

- Add a six-state-bit washing-machine physical plant with nondeterministic door,
  imbalance, and motor-failure events. Retain exact two-SAFE/four-UNSAFE
  agreement and a three-way shared, repeated-evidence, and checked in-process
  MTBDD reuse benchmark without relaxing the frozen controller gates.
- Add a complementary five-state-bit full-action process model, expand the
  bounded MTBDD action-output limit from four to eight, and retain exact direct
  agreement for three expected-SAFE properties using seven controller actions.

- Add a primary-source closest-system analysis for the public controller MTBDD
  result. Reject MTBDD and compile-once reuse as novelty, identify the ordinary
  in-process symbolic model as the missing strong runtime baseline, and define
  the next representative-plant trust-transfer experiment.

- Add a pinned SymbiYosys, maintained-Yosys, and Z3 oracle for both public
  washing-controller MTBDD composition answers. Match GCC's explicit zero
  initial state, reproduce the SAFE depth-32 result and the UNSAFE step-10
  result, retain machine-readable agreement, and gate it in CI.

- Add experimental BTOR2 word semantic core v1. Preserve strict 1 to 64-bit
  counter and timer expressions, exact modular evaluation, deterministic state
  transitions, constraints, bad properties, resource bounds, and a versioned
  inspection command. Unsupported BTOR2 semantics fail closed. This establishes
  a source boundary for future certificates, not a solver or novelty claim.
- Add experimental BTOR2 counter-phase certificate v1. Bind reset-or-affine
  counter traces to the exact source, compress repeated control runs into
  closed-form modular endpoints, and recheck them through a bounded verifier.
  Include accepted watchdog and actuator examples plus a rejected saturating
  near-neighbour. This remains a narrow candidate primitive without a novelty
  or production-readiness claim.
- Add the BTOR2 counter-trace exact replay portfolio. Preserve a rejected
  one-bit-input phase trace through bounded step-by-step execution, emit a
  distinct source-bound replay certificate, and verify its full final state and
  bad endpoint. The static fallback is capped at 100,000 transitions; the
  accelerated backend retains its separate trillion-transition bound.
- Add experimental BTOR2 bounded search certificate v1. Produce replayable
  `UNSAFE` input witnesses and complete-layer `SAFE` evidence for one-bit-input
  word models through bound 256. Enforce per-layer, total-state, node-step, and
  certificate-size limits. Compare both watchdog answers with pinned Bitwuzla
  unrollings and exercise the non-affine saturating model at bounds 254 and 255.
- Add exact BTOR2 word-region certificate v1 and a static bounded portfolio.
  Independently recover reset-add and reset-saturating-add recurrences from
  source, prove bounded SAFE regions arithmetically, and preserve every other
  query through explicit exact search. Retain both-answer agreement and reduce
  the 200-step actuator and 254-step saturating SAFE artifacts from 505,396 and
  802,525 bytes to 304 and 312 bytes, respectively. Arithmetic-progression
  reachability and acceleration remain established prior art; no novelty claim
  follows from this result.
- Add exact BTOR2 coupled-motion curve certificate v1 and bounded portfolio v2.
  Recognise simultaneous
  velocity and position recurrences under a shared reset, derive the complete
  bounded polynomial curve, and independently reject conjunctive motion
  envelopes without enumerating state pairs. Retain exact-search fallback for
  intersecting and semi-implicit near-neighbour models. Reduce two SAFE
  artifacts from 624,272 and 253,928 bytes to 358 bytes while explicitly
  retaining the established affine-acceleration prior-art boundary.
- Add exact BTOR2 braking-phase certificate v1 and bounded portfolio v3.
  Compose accelerate, brake, and stopped regions under every reset schedule,
  independently reconstruct phase boundaries and reachable-prefix
  completeness, and retain exact fallback for unsafe and semi-implicit
  near-neighbour cases. Reduce two SAFE artifacts from 1,180,313 and 453,342
  bytes to 386 bytes. Piecewise-affine reachability and braking invariants are
  established prior art; this is not a novelty or production-readiness claim.
- Add source-separated BTOR2 component contract v1. Bind independent
  controller, plant, and wiring sources; verify the exact feedback relation
  without constructing a monolithic BTOR2 product; and retain both-answer exact
  composed-search fallback. Preserve eight answer-balanced rows, controller
  reuse across two plants, strict hostile-input controls, CLI and Rust APIs, and
  a simulated self-service run. Retain the negative single-pair comparison:
  component proofs are 107 to 108 bytes larger and 1.35x to 1.38x slower to
  check than the monolithic specialised proofs, so no novelty claim follows.

- Add reproducible Linux evaluation bundle v1 for static x86_64 musl. Generate
  canonical archives, SPDX 2.3 SBOMs, source and lockfile provenance, capability
  snapshots, and internal SHA-256 manifests; independently verify structure,
  static linkage, identity bindings, and corruption rejection. Add a two-clone
  byte-reproducibility gate and a master-only GitHub Sigstore attestation
  workflow without publishing a release or crate.
- Retain the first signed Linux candidate evidence at master commit `47aeb69`.
  Both SLSA and SPDX attestations bind archive digest `6bb88302...01d6f` to the
  exact protected workflow, source commit, and GitHub-hosted runner. Offline
  replay verifies the archive without executing the candidate binary.

- Add the proof-carrying event-contract feasibility primitive. Rebuild named-CNF
  relation and terminal completeness obligations independently of the BDD,
  replay every concrete edge and terminal witness against the source AIG, and
  check native UNSAT proofs. Preserve 30 release-mode trials across the 9, 12,
  and 16-input product cohort, including 0.261 to 1.051 ms median verification,
  plus an omitted-target rejection regression. Certificate v3 remains open.
- Freeze and implement experimental event-contract certificate v3. Bind the AIG
  and original named contract, carry edge witnesses and checked completeness
  proofs, independently recompute powered phases and final composition, replay
  positive traces, and prove negative answers. Preserve 40 cost trials spanning
  both answer classes, including the negative 2.26x to 7.23x verification
  overhead against exact CDCL, plus bounded parser and tamper rejection evidence.
- Export every v3 relation and terminal completeness claim to deterministic,
  source-bound DIMACS bundles. Verify all 68 individual obligations and four
  selector aggregates with pinned CaDiCaL 3.0.0 and DRAT-trim, covering both
  answer classes under explicit process and proof-file limits.
- Add release-candidate event-contract CLI v1 and typed `EventContractTool`
  discovery, production, checking, portfolio, report replay, execution bounds,
  and invocation metrics without changing predicate API v1.
- Add a timing-free structural v3 admission rule with exact persistent-CDCL
  fallback on rejection or recognized bounded resource exhaustion. Preserve 60
  answer-balanced product-shaped trials with exact agreement and the negative
  2.34x to 12.73x verification overhead.
- Add a portable six-case self-service acceptance harness and retained simulated
  acceptance evidence. This is workflow evidence, not independent partner
  validation.

- Add experimental bounded event-contract v1 semantics: strict named CNF over
  relevant AIGER inputs, exact BDD phase composition, concrete witness recovery,
  direct-AIG replay, and a separately encoded exact CDCL control. Preserve
  30 release-mode product trials and the negative 1.09x to 36.20x query overhead
  instead of admitting the backend universally. Add canonical parser bounds,
  hostile-input rejection, three firmware/robotics contracts, and a
  no-overwrite reproduction script.

- Position the project as Guarded Continuation Checker, powered by CQ-SAT. Add
  a collision-aware brand architecture, original guard-aperture SVG identity,
  partner-facing naming transition and canonical repository URL. Before the
  first published crate, rename the Rust package, library and executable from
  the pre-release research name to `guarded-continuation-checker`.

- Add deterministic predicate-v2 obligation bundle v1 export: source- and
  certificate-bound canonical DIMACS for every relation/terminal completeness
  claim plus a selector-guarded aggregate that is UNSAT exactly when every
  constituent obligation is UNSAT.
- Add a pinned, resource-governed CaDiCaL 3.0.0/DRAT-trim external baseline.
  Preserve all 40 individual and four aggregate Ubuntu checks, record the
  binary-proof interoperability finding, and retain the negative performance
  result rather than claiming external checking is faster.

- Add authoritative production-readiness and novelty gap registers, with
  measurable closure gates and a scoped proof-carrying predicate-composition
  candidate contribution grounded against closest certifying-model-checking
  methods.
- Freeze the candidate dense predicate certificate v1 contract, including
  deterministic phase relations, source binding, positive trace evidence,
  negative terminal-set evidence, an independent exhaustive checking algorithm
  and explicit fail-closed limits.
- Add the certificate verifier's independent trusted core: separate AIG support
  analysis, exhaustive transition/property evaluation, one-step relations,
  deterministic powers, composition and terminal safe-state reconstruction,
  cross-checked against the BDD producer on all three product controllers.
- Add strict self-service `certify-aiger-predicate` and
  `verify-aiger-predicate-certificate` commands for positive and negative
  certificate v1 results, with deterministic source/phase/terminal evidence,
  direct-AIG replay, atomic publication and structural, semantic, canonical-text
  and symlink tamper rejection.
- Add counterfactual portfolio v1: a no-overwrite self-service command and
  versioned report that selects independently verified predicate certificates
  through the calibration-free static gate, preserves exact persistent-CDCL
  fallback on rejection or bounded predicate failure, and regression-tests
  both avoidable and unavoidable fallback answers.
- Add the strict `verify-aiger-counterfactual-report` consumer, which recomputes
  admission and dimensions, binds the complete transcript to independently
  checked predicate evidence, and re-solves CDCL reports. Restrict fallback to
  explicit resource errors, add deterministic resource-exhaustion coverage,
  canonical-report rejection tests, and exercise the admitted certificate
  contract across all three product-shaped controllers.
- Add an answer-balanced certificate-cost benchmark with source/transcript
  bindings and ten preserved release trials per case. Record the negative
  evidence that proof publication costs 10–13 ms and exhaustive checking reaches
  136 ms at 16 inputs despite sub-1-KiB certificates, establishing a concrete
  non-enumerative-checker target without claiming an external-tool comparison.
- Add a certificate-v2 feasibility experiment that proves every one-step
  relation through concrete edge witnesses and native Varisat UNSAT completeness
  proofs checked by `varisat-checker`. Preserve ten trials across the 9/12/16
  input cohort, including a 280.32x checker speedup at 16 inputs, proof-size and
  generation tradeoffs, truncation rejection and omitted-edge detection.
- Extend the v2 feasibility result to terminal safe-state evidence: concrete
  safe witnesses plus one checked UNSAT completeness proof, omitted-safe-state
  detection, and six preserved unconstrained/constrained cohorts. Retain the
  negative easy-terminal rows and the 26.20x constrained 16-input speedup.
- Add the bounded canonical predicate certificate v2 producer and independent
  verifier. Bind ordered edge witnesses, per-source native UNSAT proofs,
  deterministic powered phases, terminal witnesses/proof and the final trace;
  enforce individual/aggregate proof and artifact limits; cover deterministic
  output, both answer classes, all product widths, changing phases and semantic,
  proof, source, ordering and canonical-text tampering. Keep v1 as the portfolio
  format pending v2 hardening and cost gates.
- Add an answer-balanced canonical-v2 cost harness and ten preserved release
  trials per row. V2 reduces the 16-input end-to-end independent check from
  136.045 ms to 0.831 ms (163.71x), while honestly retaining its 33–39 ms
  production cost, 9.7–52.1 KiB artifacts and 1.38–5.89x verification overhead
  against the exact CDCL control.
- Add the certificate-v2 reliability boundary and deterministic robustness
  corpus: 5,000 parser transformations, bounded-file/proof cases and 128 native
  proof transformations. Structurally preflight proof integers, dimensions,
  lists and termination before the third-party checker and convert unexpected
  dependency failures into fail-closed verification errors.
- Freeze predicate CLI contract v1 with a strict machine-readable version query
  declaring certificate/proof formats and bounded dimensions. Stabilise command
  arguments and exit meanings, distinguish the portfolio's v1 format from
  explicit v2 operation, and define migration plus a multi-release deprecation
  window.
- Add predicate Rust API v1: a typed, shell-free client that discovers and
  validates CLI capabilities, separates avoidable/unavoidable results from
  operational failures, and produces or checks certificate v1/v2 artifacts.
  Exercise it as a separate downstream-style integration-test crate against the
  real built executable and product fixture.
- Add configurable per-invocation governance to predicate Rust API v1: a
  five-minute/1-MiB default, validated custom deadlines and 1-B–64-MiB stream
  caps, concurrent bounded output collection, and stable typed timeout and
  output-limit failures. Preserve operating-system memory/process-tree controls
  as an explicit remaining deployment gate.
- Add predicate invocation metrics schema v1. Observed discovery,
  certification and verification return duration, stream sizes, configured
  limits, exit status and a stable failure class on both success and error
  paths, with canonical privacy-preserving CSV rows for build/fleet aggregation.
- Extend predicate API governance to the operating-system boundary. Unix jobs
  run in isolated process groups with complete-group deadline termination and
  configurable file ceilings; supported non-macOS Unix targets add a 2-GiB
  default configurable address-space ceiling. Metrics report which controls
  were enforced, while macOS remains explicitly unavailable for hard memory
  evidence.

## 0.28.0 - 2026-07-18

- Add an experimental bounded exact BDD predicate interface for 9–16 relevant
  AIGER inputs, powered temporal relation composition, original-AIG trace
  replay, a dense 16-sensor fixture, and a reproducible reuse sweep. Low-reuse
  results remain negative; 100 and 1,000 reuses show positive amortisation. Add
  a maintained-Yosys existential bounded-query agreement baseline with
  separately labelled process-level timing.
- Add three state-dependent product fixtures spanning 9, 12, and 16 relevant
  inputs, reproducible RTL-to-AIGER synthesis, and a 120-row constrained
  temporal matrix against persistent CDCL and Yosys. Preserve the negative
  short-horizon actuator/sensor-fusion results as an admission boundary.
- Add a timing-free conservative predicate admission rule over support width,
  latch count, horizon, and expected query reuse; it excludes every observed
  robust loss regime and records backend eligibility without timing
  calibration. Portfolio integration remains separately gated.
- Add dual-direction cross-backend exactness coverage: reconstructed avoidable
  traces replay against the original AIG, while fixed unsafe transcripts must
  be reported unavoidable by the predicate quotient, persistent CDCL, and
  maintained Yosys.

## 0.27.0 - 2026-07-18

- Add exact AIG cone-support projection for CIQ, admitting up to 64 declared
  firmware or robotics inputs when no more than eight affect the combined
  transition/property interface, with full-input witness lifting and fail-closed
  dense-support rejection.
- Add a 16-input mobile-robot obstacle-stop fixture, exhaustive projection
  tests, independent causal replay, a reproducible ten-trial scaling harness,
  and bounded evidence showing 2.46x–10.74x median end-to-end speedups.

## 0.26.0 - 2026-07-18

- Add an exact Counterfactual Interface Quotient experiment for small
  input-driven firmware controllers, with interval-preserving powered relation
  summaries, semantic no-op suppression, on-demand trace reconstruction, and
  identical persistent-CDCL causal transcript replay.
- Add an independent strict report verifier, source/coverage/tamper checks,
  explicit resource bounds, adversarial tests, and a no-overwrite scaling
  regeneration harness.
- Add a ten-trial horizon-scaling result showing a 1.42x to 7.00x median
  end-to-end speedup on the bounded infusion-pump regression, document the
  falsified leaf-expansion and explicit-wide-input variants, and define the
  firmware/robotics research roadmap and novelty boundary.

## 0.25.0 - 2026-07-18

- Add strict original-format binary AIGER (`aig`) ingestion to every AIGER
  command, with bounded delta decoding and semantic parity tests against ASCII.
- Add a compile-once causal batch experiment over every reachable bounded
  failure target, three observation vocabularies, bounded enumeration of
  distinct 1-minimal causes, identical persistent-CDCL/CQ transcript replay,
  break-even reporting, atomic publication, and an independent cause verifier.
- Tighten the causal-analysis novelty boundary against prior work on minimal
  BMC assignments, causal counterexample explanation, compiled Boolean
  explanation, and interval-based counterexample analysis.

## 0.24.0 - 2026-07-18

- Add a bounded closest-method causal benchmark comparing ordered deletion and
  QuickXplain over identical fresh-CDCL transcripts replayed through persistent
  CDCL and admitted continuation quotients.
- Add exhaustive small monotone-oracle correctness tests, controlled sparse and
  dense AIGER fixtures, an eight-row reproducible result, and claim-bounded
  prior-art documentation.

## 0.23.0 - 2026-07-18

- Add bounded, certificate-producing causal analysis for input-driven AIGER
  counterexamples, with exact 1-minimal sufficient-cause semantics and an
  evidence-replaying certificate verifier and atomic, no-overwrite v1 bundle.
- Compare continuation-quotient intervention reuse against fresh and persistent
  CDCL on the identical minimisation sequence, with conservative admission and
  work limits and explicit query-only versus amortised measurements.

## 0.22.0 - 2026-07-17

- Rewrite external communication in the individual maintainer's first person
  and make design-partner and assessor evaluations explicitly self-service.
- Remove any maintainer engineering-support or private-evidence expectation and
  add a strict non-confidential outcome/suitability report as the only intended
  evaluation artifact returned to the maintainer.

## 0.21.0 - 2026-07-17

- Add sendable, claim-bounded design-partner outreach and a controlled
  engagement sequence for confidential representative RTL pilots.
- Add a private pilot intake/closeout template and an independent security and
  technical assessment statement of work tied to the executable evidence gate.

## 0.20.0 - 2026-07-17

- Add a fail-closed external-evidence register validator and production-gate
  checker for schema, result/exit agreement, cohort diversity and size,
  independent review coverage, replay, repetition, resources, findings, and
  attestation requirements.
- Exercise valid, disagreement, exit mismatch, witness replay, unresolved row,
  spreadsheet-injection, attestation, cohort-size, symlink, annotated-tag, and
  tag/commit binding paths in CI without populating the canonical header-only
  evidence register.

## 0.19.0 - 2026-07-17

- Define fixed independence, attack reproduction, technical review, partner
  cohort, oracle comparison, evidence-register, and acceptance requirements for
  closing the remaining production gates without moving the goalposts.
- Add the canonical header-only external-evidence register schema without
  inventing partner or review results.

## 0.18.0 - 2026-07-17

- Define CQ-SAT/GCC's exact bounded, model-relative assurance claim and its
  exclusions for physical-product and lifecycle safety.
- Add an applicability matrix, adopter evidence responsibilities, and permitted
  wording for ISO 26262, IEC 61508, IEC 62304, FDA infusion-pump guidance, and
  IEC 81001-5-1 without implying conformity, certification, or qualification.

## 0.17.0 - 2026-07-17

- Add hostile-RTL isolation profile v1 with an immutable Yosys image, no
  network, read-only root/input mounts, non-root execution, zero capabilities,
  `no_new_privileges`, seccomp, and bounded cgroup-v2 resources.
- Probe every effective isolation control before parsing RTL, preserve SAFE and
  UNSAFE exit semantics, validate evidence in a second read-only container, and
  record a sibling isolation report. Enforce a container-ID-based 300-second
  outer watchdog plus 30-second probe and validation deadlines.

## 0.16.0 - 2026-07-17

- Add a production-evaluation operations runbook covering supported hosts,
  installation, qualification, upgrades, rollback, monitoring, incident
  response, restoration drills, support ownership, and evidence retention.
- Add a Linux-only fail-closed qualification script that exercises known SAFE
  and UNSAFE RTL, schema-v4 validation, CLI contracts, and containment fields;
  require it in CI.

## 0.15.0 - 2026-07-17

- Add artifact schema v4 with a bounded SHA-256 inventory covering generated
  RTL, synthesis, model, solver, and report evidence; reject tampering and
  symlink substitution, with 512 MiB per-file and 2 GiB aggregate validation
  limits.
- Document the deployment threat model and remaining Yosys isolation limits.
- Pin GitHub Actions to immutable commits, require RustSec dependency auditing
  in CI, and enable weekly Cargo and Actions dependency updates.

## 0.14.0 - 2026-07-17

- Add a revision-pinned public RTL corpus built from five unmodified Yosys test
  sources and twelve CQ-owned SAFE/UNSAFE properties, with exact provenance and
  executable checksum verification.
- Require all twelve cases to match their expected result on both current Yosys
  and a digest-pinned Yosys 0.36 image; independently cross-check the current
  run with SymbiYosys/Z3.
- Normalize parameterless project evidence as `parameters=none` and add generic
  technology mapping before AIG lowering, fixing corpus-discovered compatibility
  failures for parameterless designs and dynamic memory indexing.

## 0.13.0 - 2026-07-17

- Add project config v2 startup-reset sequences with exact asserted-frame and
  deasserted-frame constraints in every bounded AIGER query.
- Cross-check the reset boundary independently with SymbiYosys/Z3 and preserve
  the full reset pattern in safety-report and manifest evidence.

## 0.12.0 - 2026-07-17

- Add strict RTL project config v1 with immutable include snapshots, bounded
  top-parameter overrides, declared clock/reset policy, and memory lowering.
- Add artifact schema v3 and firmware CLI v2 evidence for project semantics,
  plus a parameterised infusion-pump memory model cross-checked by SBY/Z3.
- Extend deterministic parser mutation coverage to strict project configs.

## 0.11.0 - 2026-07-17

- Establish RTL artifact schema v2 as the first compatibility-locked evidence
  contract and add a strict bundle validator for field, status, and snapshot
  consistency.
- Establish firmware CLI contract v1 with a machine-readable version query,
  fixed command signatures, and stable exit meanings.
- Bound direct ASCII AIGER ingestion to 256 MiB before and after reading.
- Add 20,000 stable-Rust parser and CLI mutation cases backed by persistent
  malformed and valid regression corpora.

## 0.10.0 - 2026-07-17

- Run Yosys in a dedicated Unix process group with a 512 MiB output-file cap,
  kill the complete group on timeout, and enforce a 2 GiB address-space limit
  on Linux.
- Record the effective containment platform and limits in safety reports and
  manifests, explicitly reporting memory containment as unavailable on macOS.

## 0.9.0 - 2026-07-17

- Add fail-closed, named RTL input assumptions that constrain every bounded
  frame, preserve their source artifact, and reject duplicates or unknown names.
- Cross-check constrained SAFE semantics independently with SymbiYosys and Z3
  while retaining the matching unconstrained UNSAFE regression.

## 0.8.0 - 2026-07-17

- Add a bounded multi-file RTL project safety gate with deterministic source
  staging, duplicate detection, aggregate limits, and manifest provenance.
- Remove stale source snapshots before atomic manifest publication so reruns
  cannot mix evidence from different project inputs.
- Mark RTL safety reports and manifests with artifact schema version 1.

## 0.7.0 - 2026-07-17

- Flatten hierarchical modules before RTL-to-AIGER export and make synthesis
  don't-care lowering explicit, enabling realistic multi-module controllers.
- Add an exact repeated-property BMC benchmark with bounded two-query reuse,
  cold-solver agreement, and a static no-regression portfolio gate.
- Add a five-module infusion-pump system and curated horizon-scaling evidence.

## 0.6.0 - 2026-07-17

- Add an RTL-to-safety-gate path that synthesizes bounded SystemVerilog through
  Yosys into the exact supported ASCII AIGER subset.
- Preserve Yosys input, latch, and bad-output names in human-readable traces;
  publish source, synthesis, model, solver, provenance, and manifest artifacts.
- Replace the hand-authored product workflow with safe and regressed
  SystemVerilog controllers cross-checked independently by SymbiYosys and Z3.

## 0.5.0 - 2026-07-17

- Add a product-shaped firmware safety gate with CI-specific exit statuses,
  GitHub Actions annotations, stable report artifacts, and a copyable workflow.
- Add safe and deliberately regressed infusion-pump controller models that show
  build acceptance and shortest-trace failure reproduction end to end.

## 0.4.0 - 2026-07-17

- Add exact primary-input and wider-model AIGER bounded model checking using a
  scalable Tseitin-unrolled CDCL fallback selected without trial solving.
- Combine all bad outputs and frames into one safety query, then minimize unsafe
  traces to the shortest bad horizon while preserving complete input witnesses.
- Add revision-pinned Peterson mutual-exclusion and SPI receiver models covering
  real SAFE protocol verification and UNSAFE hardware input-trace reconstruction.

## 0.3.0 - 2026-07-17

- Add a validated ASCII AIGER import path for closed deterministic safety models,
  including initial latch values and bad-state reachability queries.
- Add an independently sourced, revision-pinned four-bit counter-overflow model
  with its upstream MIT license and an executable portfolio workflow.
- Extend the static gate with query assumption density after the external model
  exposed a full-state-query counterexample to density-only admission.

## 0.2.0 - 2026-07-17

- Add the bounded, calibration-free CQ-SAT/GCC portfolio gate with exact
  persistent-CDCL fallback and declared-query amortization thresholds.
- Add unseen majority, multiplexer, and mixed-dynamics holdouts plus independent
  query-seed stability checks.
- Add executable watchdog/interlock and redundant sensor-voting verification
  examples showing specialized and fallback decisions.

- Add a bounded-width temporal model-checking phase benchmark.
- Add an exact repeated-transition kernel with full witness reconstruction.
- Preserve dense-quotient negative results alongside kernel measurements.
- Recognize a fixed deterministic transition vocabulary directly from layered CNF.
- Replace repeated template scans with one-pass normalization and logarithmic
  transition jump tables.
- Update GitHub Actions checkout to its Node 24 release.
- Recognize arbitrary total deterministic repeated transitions within a fixed
  width gate, including compositions outside the named rule vocabulary.
- Recover separable output functions locally from repeated CNF, eliminating the
  exhaustive current/next state-pair scan while preserving complete witnesses.
- Replay recovered local transition functions without an explicit `2^width`
  state table for fully specified deterministic initial states.
- Solve partial-initial-state temporal queries with exact BDD preimages and full
  witness reconstruction under a hard node-budget admission gate.
- Add calibration-free natural, reverse, even/odd, and dependency-graph BDD
  orders; preserve the negative symmetric-ring comparison and gated holdout.
- Add asymmetric hub, tree, and irregular transition graphs; dependency ordering
  reduces aggregate BDD size on phase and unseen holdout cohorts.
- Detect exact repeated symbolic frames and reuse transient/cycle checkpoints for
  long-horizon preimage queries without redundant BDD composition.
- Add an optional calibration-free BDD growth guard that rejects projected
  pre-cycle budget exhaustion early without approximating an answer.
- Add an exact hybrid backend that switches growth-guard cases from symbolic BDD
  preimages to persistent CDCL, restoring complete workload admission.
- Add an exact BDD-prefix-to-CDCL checkpoint experiment and preserve its negative
  performance result for naïve Tseitin encoding.
- Add a structurally hashed AIG checkpoint encoding and preserve the finding that
  it expands, rather than compacts, the measured cascade prefix.
- Add exact lazy observation-cone checkpoint encoding with direct BDD-root
  assumptions and prefix witness reconstruction.
- Expand cyclic symbolic frames correctly when checkpoint encodings reference a
  frame beyond the stored transient/cycle vocabulary.
- Add an exact native BDD-theory/CDCL bridge with activation-gated conflict
  learning and bounded pairwise theory propagation.
- Generalize rejected checkpoint states into exact BDD-proven incompatible
  subcubes and report learned-clause width.
- Add prefix/suffix conjunction caches for linear-pass exact BDD conflict
  explanation extraction.
- Precompile bounded exact global checkpoint-image clauses for reuse across all
  native BDD-theory queries and report recognition-inclusive break-even.
- Validate reusable global clauses across asymmetric hub, tree, and irregular
  transition families at widths 7, 9, and 11.

## 0.1.0 - 2026-07-15

- Initial research release.
- Exact continuation quotient construction and witness recovery.
- Conservative structural gate and full frontier profile.
- Reusable assumption-query benchmark against persistent Varisat.
- Exact local clause insertion repair.
- Provenance-safe root rebuild for clause deletion.
- DIMACS/SATLIB evaluation path.
- Curated positive, negative, and corrected results.
