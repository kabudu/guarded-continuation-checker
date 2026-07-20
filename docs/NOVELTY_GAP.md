# Novelty gap register

No scholarly novelty claim has been established for CQ-SAT's dense
predicate quotient. Exact BDD representation, symbolic model checking,
counterexample witnesses and model-checking certificates all have substantial
prior art. The candidate contribution must therefore be narrower and tested
against the closest methods.

## Candidate contribution

The working hypothesis is:

> A deterministic, independently checkable certificate can connect bounded
> input-predicate projection, powered phase composition, static portfolio
> admission and concrete AIGER witness recovery for repeated firmware
> counterfactual queries.

This is a hypothesis, not a claim. It is novel only if the combined certificate
semantics and operational use are absent from prior systems and provide a
measurable capability beyond straightforward composition of known techniques.

## Closest known areas

| Area | Why it is close | Required comparison |
|---|---|---|
| AIGER witness semantics | Standardises bad-state traces and witness checking | Emit or translate compatible concrete traces and distinguish them from proof certificates |
| Certifaiger and k-witness circuits | Certifies positive hardware model-checking results through independently checked obligations | Compare trust base, certificate size, supported answer classes and checking cost |
| General proof certificates for finite-state exploration | Defines proof evidence and trusted checker frameworks | Map each predicate-composition obligation to the framework or explain the additional semantics |
| BDD symbolic model checking | Already represents and composes transition relations symbolically | Show that BDD use itself is not claimed as novel |
| Interactive BDD certification | Recent self-certifying CTL model checking uses probabilistic interactive verification | Compare deterministic versus probabilistic checking, property scope and certificate cost |
| SAT/QBF proof and function certificates | Provides independently checked resolution or Skolem/Herbrand evidence | Determine whether existing proof formats can encode our obligations more directly |

Primary starting points:

- [AIGER format and witness ecosystem](https://fmv.jku.at/aiger/)
- [Progress in Certifying Hardware Model Checking](https://fmv.jku.at/papers/YuBiereHeljanko-CAV21.pdf)
- [Certifaiger](https://fmv.jku.at/certifaiger/)
- [A framework for proof certificates in finite state exploration](https://arxiv.org/abs/1507.08716)
- [iSMC: A BDD-based Symbolic Model Checker with Interactive Certification](https://arxiv.org/abs/2605.03705)
- [QBFcert](https://fmv.jku.at/qbfcert)

The [bounded prior-art audit v1](PRIOR_ART_AUDIT_V1.md) found materially closer
disconfirming work than the original list captured. In particular, FMCAD 2023
already composes certificates produced by BDD and other model-checking engines
across temporal decomposition, while FM 2026 composes arbitrary witness
circuits and shares certificate checks across multiple properties. Patent
records also describe SAT-derived predicate transition enumeration, proof-based
abstraction, and abstract hardware/software composition. The broad combination
of BDDs, predicate projection, composition, and independently checked evidence
is therefore not a credible novelty claim. Gate 5 remains open because this was
not a systematic or professional patent search.

## Claim gates

1. **Closed for candidate v1:** freeze a precise certificate language and
   trusted-checker semantics.
2. **Closed for candidate v1:** implement the checker independently from the
   producer's BDD and cache code.
3. **Closed for candidate v1:** demonstrate rejection of structural, semantic,
   ordering, truncation and source-binding tampering.
4. **Partially closed:** compare certificate generation/checking cost and trust
   base with the closest maintained certifying tools. The obligation-equivalent
   CaDiCaL/DRAT-trim comparison is complete. The whole-certificate Certifaiger
   interface, semantic-equivalence, resource, hostile-control, and hosted
   replication gates pass on the frozen corpus. The result is negative on
   speed, size, and packaging. A low-memory GCC verifier profile reproduces on
   arm64 and amd64, and both evidence paths reproduce byte-identical artifacts
   across architectures. Certifaiger's SAFE obligations are checked through
   CaDiCaL and the formally verified `lrat_isa` checker. The remaining gate is
   a direct comparison with the FM 2026 composed multi-property witness, not a
   missing formally verified SAT-proof consumer.
5. Search papers, tools, patents and current implementations for the complete
   proposed combination, recording both confirming and disconfirming evidence.
6. Obtain external expert review of the scoped claim.

Until all six gates close, repository and outbound language must say “candidate
contribution” or “research prototype”, never “novel breakthrough”.

The revised candidate question is whether exact controller evidence can remain
reusable across separately supplied and changing plant contracts while a
bounded low-memory checker proves quotient completeness, wiring, and both
answer classes without rebuilding a whole-circuit inductive witness. This must
be compared directly with the FM 2026 composed-witness construction under the
[predeclared baseline](COMPOSED_WITNESS_BASELINE_V1.md). Until that comparison
demonstrates a distinct semantic capability or measured property, it is a
research direction rather than a candidate contribution.

Gate 1 is represented by the frozen
[`Dense predicate certificate v1`](PREDICATE_CERTIFICATE_V1.md), with working
producer and independent exhaustive verifier implementations for both answer
classes. This closes only the specification gate, not the novelty claim.

The internal [certificate cost experiment](PREDICATE_CERTIFICATE_COST.md)
preserves ten trials for both answer classes. V1 records a strong negative
result for exhaustive proof checking; canonical v2 removes that bottleneck and
cuts the 16-input check by 163.71x, but remains slower than CDCL and increases
production and artifact cost. It informs gate 4 but does not close it because an
obligation-equivalent maintained external certifying-tool comparison has not yet
been run.

The [proof-carrying one-step relation experiment](PREDICATE_PROOF_RELATION_EXPERIMENT.md)
uses concrete edge witnesses plus checked UNSAT completeness obligations. It
removes the measured exponential verifier bottleneck, but SAT proof carrying is
established prior art. Any candidate novelty remains in the complete bounded
predicate-composition contract and embedded counterfactual workflow, not in
this proof technique alone.

The candidate primitives are now integrated into a bounded canonical
[certificate v2](PREDICATE_CERTIFICATE_V2.md) with deterministic powered-phase
composition and terminal trace semantics. This strengthens the candidate
combination but does not close prior-art search, whole-certificate external-tool
comparison or expert-review gates.

The [external predicate-proof baseline](EXTERNAL_PREDICATE_PROOF_BASELINE.md)
exports every v2 completeness claim to canonical DIMACS, checks 40 individual
obligations and four selector-guarded aggregates with pinned CaDiCaL 3.0.0 and
DRAT-trim, and records a negative performance result: external checking takes
33.702--57.671 ms per aggregate versus 0.311--0.882 ms for native end-to-end v2
checking. The aggregate construction is a standard guarded disjunction and is
not claimed as novel. It adds implementation-diverse evidence and closes the
obligation-equivalent SAT-proof portion of gate 4, not the whole-certificate
Certifaiger or expert-review portions.

The [bounded event-contract experiment](EVENT_CONTRACT_EXPERIMENT.md) adds
strict named CNF assumptions to powered predicate composition and exact witness
recovery. CNF assumptions, contracts, BDD composition, bounded model checking,
and witness replay all have established prior art. The experiment emits no new
certificate and has no portfolio integration, so it does not establish novelty.
Its candidate value is only as one component of the narrower combined artifact
described above. The retained three-product timing result is entirely negative.

The [proof-carrying event-contract primitive](EVENT_CONTRACT_PROOF_EXPERIMENT.md)
shows that CNF-constrained relation and terminal obligations can be checked
without trusting the producer BDD. SAT proof carrying and witness-backed
completeness are established techniques, so the measured checker result is not
itself novel. It lowers implementation risk for the candidate combined artifact
but does not close prior-art search, whole-certificate comparison, or review.

Experimental [event-contract certificate v3](EVENT_CONTRACT_CERTIFICATE_V3.md)
now binds the original named contract to independently checked CNF relations,
powered composition, terminal evidence, answer, and trace. This is the most
complete implementation of the candidate combination in this repository, but
combining known methods is not enough to establish novelty. External standard
proof checking, search for the complete combination, and expert review remain
open claim gates. Repository language must continue to say candidate
contribution or research prototype.

The [external event-contract proof baseline](EXTERNAL_EVENT_CONTRACT_PROOF_BASELINE.md)
now verifies every v3 completeness obligation with maintained CaDiCaL and
DRAT-trim implementations. This closes proof-format and checker diversity for
that layer. It does not establish novelty for the combined certificate,
externally check composition or replay, or replace the remaining prior-art and
expert-review gates.

Event-contract CLI/Rust API v1 and its structural portfolio make the candidate
combination independently usable without formula-specific calibration. This is
productisation evidence, not a novelty result. Exact fallback, typed discovery,
resource controls, and answer balance do not by themselves distinguish the
underlying algorithm from established symbolic model checking, proof-carrying
SAT, contract checking, or solver portfolios.

The experimental [BTOR2 word semantic core v1](BTOR2_WORD_CORE_V1.md) preserves
bounded counter and timer expressions before bit blasting. BTOR2 parsing,
bit-vector evaluation, bounded model checking, and word-level SMT solving are
established prior art. The core does not change any novelty gate. A future claim
would require proof-carrying word-level composition beyond BTOR2Tools, BtorMC,
Bitwuzla, and straightforward combinations of existing certificate methods.

The experimental [counter-phase certificate v1](BTOR2_COUNTER_PHASE_CERTIFICATE_V1.md)
adds exact source binding and closed-form verification for a strict
reset-or-affine counter shape. Its ingredients are established and its current
combination has not been shown novel. The open novelty test is whether a wider
source-bound composition certificate can provide independently measured
assurance or scaling beyond straightforward recurrence acceleration plus SMT.

[BTOR2 bounded search v1](BTOR2_BOUNDED_SEARCH_V1.md) adds proof-carrying exact
reachability for both answers, but does so with established explicit reachable
layers and witness replay. It is an exact reference backend and integration
boundary, not a novelty result. Novelty remains open until a compressed
word-composition certificate proves the same successor completeness obligation
without enumerating the measured state layers.

[BTOR2 exact word-region certificate v1](BTOR2_WORD_REGION_CERTIFICATE_V1.md)
closes that narrow measured compression target: independently recovered
arithmetic progressions reduce the two large SAFE artifacts by more than 99.9%
and a static portfolio preserves explicit exact fallback. Arithmetic-progression
descriptions of one-counter reachability, recurrence acceleration, symbolic
reachability, and word-level BMC are established prior art. This result is a
useful proof-carrying integration primitive, not a novel algorithm. The open
candidate question moves to exact composition of interacting word regions with
a certificate materially different from straightforward products of known
counter accelerations.

[BTOR2 coupled-motion curve certificate v1](BTOR2_MOTION_CURVE_CERTIFICATE_V1.md)
implements the first interacting-state case. It preserves the exact polynomial
relation between velocity and position, reduces two large SAFE artifacts by
more than 99.8%, and rejects a recurrence near-neighbour through exact fallback.
Triangular affine recurrences, discrete kinematics, counter and affine-system
acceleration, relational invariants, and safety certificates are established
prior art. This is useful engineering evidence, not a novel algorithm. The
remaining candidate boundary requires phase-composable relations across
multiple control inputs or components, plus a complete closest-system search
and external expert review.

[BTOR2 braking-phase certificate v1](BTOR2_BRAKING_PHASE_CERTIFICATE_V1.md)
composes accelerate, brake, and stopped regions under arbitrary reset schedules
and compresses two complete SAFE artifacts by more than 99.9%. Piecewise-affine
reachability, braking invariants, recurrence acceleration, arithmetic-series
summaries, and independently checked safety certificates are established prior
art. This result is not claimed as novel. The candidate boundary now requires
composition across separately sourced controller and plant contracts, with a
non-Cartesian interface relation and maintained product or SMT baselines.

[BTOR2 source-separated component contract v1](BTOR2_COMPONENT_CONTRACT_V1.md)
implements that source boundary, binds the wiring relation, preserves exact
both-answer fallback, and reuses one controller source with two plants. The
single-pair novelty hypothesis is falsified: its specialised artifact is 107 to
108 bytes larger and checks 1.35x to 1.38x slower than the equivalent monolithic
specialisation. Assume-guarantee contracts, simulation relations, compositional
invariance, witness circuits, and BTOR2 certificates are established prior art.
No novelty claim is made. The candidate now requires independently reusable
controller-local obligations that materially beat repeated monolithic checking
and straightforward certificate bundling. The first
[controller-obligation reuse specification](BTOR2_CONTROLLER_OBLIGATION_REUSE_V1.md)
now fixes a stronger parse-once, shared-model baseline and implements the
standalone source-bound obligation, compact batch, independent verifier,
canonical artifact, and self-service CLI. The first local result passes the
artifact and checking gates for fully admitted batches: at 64 members the
artifact is 34.0% smaller and checking is 12.1% faster in the retained run,
with five-run checking ratios from 0.866 to 0.885. A 25% fallback control fails
the artifact gate and has no stable checking win, so universal selection is
falsified. A public unmodified product family, maintained external-tool
agreement, cross-platform replication, and external review remain unproven.
No novelty claim is made.

The seven-action complete-cycle follow-up retains a 254-node MTBDD and exact
agreement on three horizon-64 process properties. It strengthens product-shaped
coverage but does not alter the closest-system conclusion: MTBDD compilation
and reuse are established, and the physical plant is repository-authored.

The maintained single-session SymbiYosys/Yosys/Z3 oracle now exactly reproduces
all six disturbance-aware answers and four shortest bad frames. This removes
the external semantic agreement gap for the fixture, but it is validation
evidence rather than novelty evidence. Comparable memory measurement and a
non-repository-authored physical environment remain open.

The seven-action complete-cycle follow-up retains a 254-node MTBDD and exact
agreement on three horizon-64 process properties. It strengthens product-shaped
coverage but does not alter the closest-system conclusion: MTBDD compilation
and reuse are established, and the physical plant is repository-authored.

The next [dense controller relation v1 experiment](DENSE_CONTROLLER_RELATION_V1_EXPERIMENT.md)
tests a broader candidate boundary: reuse the existing witnessed-edge and UNSAT
row-completeness proof primitives as one source-bound controller relation across
several environments. Finite relations, relational composition, reusable
assumptions, and proof-carrying model checking are prior art. Only a measured
win on revision-pinned, unmodified embedded RTL with strong ordinary baselines
could narrow the novelty gap. No such result exists yet.

The first [proof-carrying controller transducer v1](PROOF_CARRYING_CONTROLLER_TRANSDUCER_V1.md)
implementation now retains sensed-input to next-state and actuator-output
correlation, proves every symbolic cell complete, checks a source-bound
canonical artifact independently, composes it with sampled plants, and agrees
with an independent direct-controller baseline across the retained small query
grid. Its first complete canonical batch artifact passes the local strong reuse
baselines: at 64 members it is 71.5% smaller and checks 74.5% faster than one
complete independently verified artifact per member. Symbolic transducers, cube
partitions, SAT proof logging, and synchronous product exploration are
established. A synthetic one-bit win closes an implementation and baseline gap,
not the novelty gap. An unmodified public embedded controller, maintained
external-tool agreement, closest-system comparison, and expert review remain
open. No novelty claim is made.

The first revision-pinned public-controller attempt is recorded in
[public washing-controller experiment v1](PUBLIC_WASHING_CONTROLLER_EXPERIMENT_V1.md).
The unmodified GPL-2.0 source synthesizes to six latches, but all 64 states are
reachable and its exact input-response relation needs 1,028 canonical cube
leaves. It is rejected by the frozen 256-cell gate before proof production.
This falsifies straightforward state-limit expansion for that candidate and
points toward non-cube equivalence predicates or a cleaner public controller.
It supplies no novelty evidence.

The fixed state-first
[controller MTBDD v1](PROOF_CARRYING_MTBDD_V1.md) then represents the identical
public relation with 254 shared decision nodes and 153 terminals, and verifies
all 131,072 assignments independently. This is a substantial internal
representation improvement over the failed cube vocabularies. Reduced ordered
multi-terminal BDDs and exhaustive equivalence checking are established, so the
result is not itself novel. The candidate novelty boundary moves to reusable
source-bound composition evidence that beats ordinary complete artifacts and
the closest maintained compositional checker on realistic environments.
The first complete-artifact reuse baseline now passes on the public controller
and repository-authored appliance monitors. At 16 members, shared bytes are
10.6% and checking time is 6.2% of repeated complete artifacts, with exact
agreement. This is meaningful reusable-certificate evidence, but BDD sharing,
model compilation, and amortized equivalence checking are established. Pinned
SymbiYosys with maintained Yosys and Z3 now reproduces both minimal composition
answers and the unsafe step-10 boundary through a separate checking path. This
removes an implementation-trust gap but supplies no novelty evidence. A
closest-system review and comparisons on representative physical environments
remain mandatory before narrowing or asserting novelty.

The primary-source
[controller MTBDD closest-system analysis](CONTROLLER_MTBDD_CLOSEST_SYSTEMS.md)
finds that MTBDD representation, compile-once symbolic reuse,
assume-guarantee composition, and proof-carrying hardware are all established.
It also identifies a weakness in the current comparison: repeated complete
artifacts are the right evidence-transfer baseline but not the strongest
runtime baseline. The next experiment must additionally compare one ordinary
compiled symbolic model reused in-process and must demonstrate a concrete
producer-consumer trust-transfer advantage. Until then, the current result is
useful engineering and not a novel algorithm or verification method.

The first [stateful physical-plant experiment](PUBLIC_WASHING_PHYSICAL_PLANT_V1.md)
passes exact agreement across six disturbance-aware properties and retains the
shared evidence advantage: 78.6% fewer boundary bytes and 70.8% less checking
time than repeated complete evidence. Shared checking is at practical parity
with GCC's checked in-process reuse in the retained run. This isolates a useful
portable-evidence result, but the in-process control uses the same GCC MTBDD
implementation. A maintained external symbolic-session cost baseline,
peak-memory measurement, and a non-repository-authored physical environment
remain open. No novelty claim is made.

Pinned SymbiYosys with maintained Yosys and Z3 now checks the six disturbance
properties in one compiled closed-loop session. It reproduces all four shortest
bad frames and preserves both bounded SAFE results. This closes external answer
agreement for the fixture, not the missing external cost comparison. Peak
memory, a non-repository-authored environment, and expert review remain open.

The versioned [controller MTBDD file workflow](CONTROLLER_MTBDD_CLI_V1.md) makes
the reusable evidence contract self-service and independently consumable.
Strict manifests, source-bound artifacts, hostile-input rejection, and
per-member reports are productisation properties, not novelty. They enable a
fair process-level cost and memory comparison but do not change the prior-art
conclusion.

That process comparison is now retained on one arm64 host. The maintained
formal route is 2.67 times faster than fresh GCC verification, while GCC uses
85.2% less peak RSS and supplies a portable 8,549-byte artifact. The negative
speed result rules out a solver-performance distinction. The lower-memory
trust-transfer result is useful engineering, but proof-carrying model checking
and portable evidence are established, so the novelty gap remains open.

The controller MTBDD plant portfolio adds timing-free selection and exact
fallback with downgrade detection. This is useful integrity engineering, but
static portfolio routing and exact fallback are established techniques. It does
not change the novelty assessment.

The proof-carrying MTBDD equivalence integration removes exhaustive controller
assignment replay from the consumer. On the public six-property physical-plant
batch it produces exact agreement and a 2.01x median end-to-end verification
speed-up, at a 29.39x artifact-size cost. This is a meaningful product trade-off
and the strongest current consumer-speed result. SAT miters, UNSAT proof
checking, and decision diagrams are established, so the result does not by
itself establish scholarly novelty. Hosted replication, established-tool
comparison on identical evidence scope, and independent expert review remain
required. The first identical-query maintained-tool baseline is negative:
SymbiYosys/Yosys/Z3 is 1.33 times faster than fresh proof verification on the
retained host. GCC uses less verifier memory and transfers independently
checkable evidence, but those properties do not overcome established
proof-carrying-hardware prior art. Hosted Linux reproduces the negative runtime
result.

The completed equivalent-evidence experiment compares GCC evidence with the
competition-standard Certifaiger and `aigsim` path at equivalent bounded scope.
Its negative runtime, size, and packaging result falsifies the broad
portable-batch advantage for this corpus. The next gate is the closer FM 2026
multi-property composed-witness baseline, which explicitly allows the narrower
controller-reuse hypothesis to be falsified.

Controller/plant resource envelope v1 converts the narrow low-memory result
into an explicit conservative consumer policy boundary. Static work estimates,
resource admission, exact fallback, and fail-closed verification are established
systems techniques. This is production hardening, not novelty evidence, and it
does not change any scholarly claim gate.

The governed proof-carrying MTBDD portfolio experiment extends that same policy
boundary to the equivalence artifact and embedded UNSAT proof. This is also
production integration, not a new algorithmic claim. Its public-product and
strong-baseline results may supply evidence for later work, but cannot close
the novelty register by themselves.

Deterministic source-to-model attestation for the public controller and plant is
reproducible-build provenance, not a new verification algorithm. It strengthens
the benchmark trust chain without changing any novelty claim or closest-prior-art
conclusion.
