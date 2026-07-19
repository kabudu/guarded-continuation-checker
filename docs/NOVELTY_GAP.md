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
