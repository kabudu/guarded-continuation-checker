# Novelty gap register

No scholarly novelty claim has been established for CQ-SAT/GCC's dense
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
2. Implement the checker independently from the producer's BDD and cache code.
3. Demonstrate rejection of structural, semantic, ordering, truncation and
   source-binding tampering.
4. Compare certificate generation/checking cost and trust base with the closest
   maintained certifying tools.
5. Search papers, tools, patents and current implementations for the complete
   proposed combination, recording both confirming and disconfirming evidence.
6. Obtain external expert review of the scoped claim.

Until all six gates close, repository and outbound language must say “candidate
contribution” or “research prototype”, never “novel breakthrough”.

Gate 1 is represented by the frozen
[`Dense predicate certificate v1`](PREDICATE_CERTIFICATE_V1.md), with working
producer and independent exhaustive verifier implementations for both answer
classes. This closes only the specification gate, not the novelty claim.
