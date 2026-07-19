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

## Claim gates

1. **Closed for candidate v1:** freeze a precise certificate language and
   trusted-checker semantics.
2. **Closed for candidate v1:** implement the checker independently from the
   producer's BDD and cache code.
3. **Closed for candidate v1:** demonstrate rejection of structural, semantic,
   ordering, truncation and source-binding tampering.
4. **Partially closed:** compare certificate generation/checking cost and trust
   base with the closest maintained certifying tools. The obligation-equivalent
   CaDiCaL/DRAT-trim comparison is complete; whole-certificate comparison with
   Certifaiger and a formally verified checker remain open.
5. Search papers, tools, patents and current implementations for the complete
   proposed combination, recording both confirming and disconfirming evidence.
6. Obtain external expert review of the scoped claim.

Until all six gates close, repository and outbound language must say “candidate
contribution” or “research prototype”, never “novel breakthrough”.

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
and straightforward certificate bundling.
