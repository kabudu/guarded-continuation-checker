# Prior-art audit v1

Date: 2026-07-20

Status: bounded maintainer research, not a legal patent opinion, freedom-to-operate
analysis, systematic literature review, or scholarly novelty determination.

## Question searched

The audit tested whether this combination is absent from prior work:

> Deterministic proof-carrying composition of dense Boolean input-predicate
> projections, powered bounded phases, static portfolio admission, and concrete
> witness recovery for repeated embedded-controller queries.

Searches covered proof-carrying and certifying symbolic model checking,
BDD-produced certificates, compositional hardware certificates, multi-property
witness reuse, predicate abstraction, SAT-derived abstract transition
relations, BTOR2 validation, embedded hardware/software composition, and patent
records using those concepts. Sources were followed to primary papers, tool
pages, and patent publications where available.

This was a keyword and citation-neighbourhood search. It does not cover every
language, non-public application, claim family, paid database, or unpublished
implementation. An expert literature review and professional patent search
remain required before any novelty or freedom-to-operate claim.

## Materially close systems

| Source | Existing capability | Consequence for GCC's claim boundary |
|---|---|---|
| [Towards Compositional Hardware Model Checking Certification, FMCAD 2023](https://froleyks.de/assets/pdf/Yu%20et%20al.%20-%202023%20-%20Towards%20compositional%20hardware%20model%20checking%20certification.pdf) | Certifies temporal decomposition by composing witness circuits. Its base engines include BDD symbolic model checking, k-induction, and IC3/PDR, and the final witness is checked externally. | Compositional certification, BDD-backed certificate production, preprocessing certificates, and external checking are prior art. |
| [Certifying Constraints in Hardware Model Checking, FM 2026](https://cca.informatik.uni-freiburg.de/papers/FroleyksYuBiereHeljanko-FM26.pdf) | Defines certificate construction for explicit and extracted constraints, implements `aigmerge` for arbitrary circuit composition, and combines per-property witness circuits into one certificate with shared reset, transition, base, and step checks. | Shared multi-property certificate composition and reuse cannot be claimed as GCC inventions. This is the closest disconfirming result found. |
| [Introducing Certificates to the Hardware Model Checking Competition, 2025](https://link.springer.com/chapter/10.1007/978-3-031-98668-0_14) | Makes machine-checkable positive certificates and counterexamples part of the HWMCC workflow, using AIGER witness circuits and Certifaiger. | Both-answer certifying hardware workflows and standard witness semantics are established. |
| [Progress in Certifying Hardware Model Checking Results, CAV 2021](https://link.springer.com/chapter/10.1007/978-3-030-81688-9_17) | Uses k-witness circuits and an independent checker reduced to SAT and a simple QBF check. | Independently checked circuit certificates and compact safety witnesses are established. |
| [iSMC: A BDD-based Symbolic Model Checker with Interactive Certification, 2026](https://arxiv.org/abs/2605.03705) | Adds probabilistic interactive certification to a BDD symbolic model checker for CTL with justice. | BDD model checking with a separate certification protocol is established, although its probabilistic and interactive trust model differs from GCC's deterministic artifact checker. |
| [A framework for proof certificates in finite state exploration, 2015](https://arxiv.org/abs/1507.08716) | Defines trusted proof-checking semantics for reachability, non-reachability, bisimulation, and non-bisimulation. | Generic proof-certificate framing and independent finite-state checking are established. |
| [Btor2-Cert, TACAS 2024](https://link.springer.com/chapter/10.1007/978-3-031-57256-2_7) | Transfers invariants between BTOR2 and software analyzers and validates safety and violation witnesses by re-verification and execution. | Source-language translation, invariant transport, and BTOR2 witness validation are established. |

## Patent records found

| Publication | Relevant disclosure | Audit treatment |
|---|---|---|
| [US7346486B2](https://patents.google.com/patent/US7346486B2/en) | SAT-based computation and enumeration of predicate-abstract transition relations, including reused common computation. | Directly disconfirms novelty of SAT-derived Boolean predicate transition enumeration and reuse in isolation. Claim scope and current legal status require professional analysis. |
| [US7406405B2](https://patents.google.com/patent/US7406405/en) | Proof-based abstraction for design verification using bounded-model-checking proof information. | Disconfirms broad claims around proof-guided abstraction in hardware verification. |
| [US8271404B2](https://patents.google.com/patent/US8271404B2/en) | SAT-assisted discovery of disjunctive and quantified invariants over predicate abstractions. | Disconfirms broad predicate-invariant discovery claims. |
| [US20170031806A1](https://patents.google.com/patent/US20170031806A1/en) | Abstracts and composes hardware and software models for embedded-system safety checking and counterexample analysis. | Disconfirms broad claims around abstract controller/software and hardware composition for safety. |
| [WO2024114920A1](https://patents.google.com/patent/WO2024114920A1/en) | Describes compositional verification of low-level drivers with hardware components. | Relevant to any future firmware/driver composition claim and requires claim-level professional review. |

Search-result similarity does not establish infringement, validity, ownership,
expiration, enforceability, or jurisdiction. No implementation decision should
be based on this table alone.

## Findings

1. The broad candidate combination is not supportable as a novel algorithm.
   Its central ingredients and several important combinations have close,
   explicit predecessors.
2. The FM 2026 multi-property witness composition is particularly close to
   GCC's shared controller proof portfolio. GCC's artifact format, resource
   envelope, attested source snapshot, exact fallback, and low-memory process
   profile are engineering differences, not yet evidence of scholarly novelty.
3. Deterministic checking distinguishes GCC from iSMC's probabilistic
   interactive protocol, but deterministic certificate checking is already
   established by Certifaiger and finite-state proof-certificate frameworks.
4. Concrete UNSAFE trace replay, SAFE certificates, portfolio routing, source
   digests, and self-service packaging are valuable product properties. Their
   assembly may be practically useful without constituting a new verification
   algorithm.
5. The retained low-memory result is a measurable implementation profile. It
   does not establish a novel method unless tied to a new certificate invariant
   and compared against the closest composed witness-circuit representation.

## Maintained-tool availability

The audit inspected the public [Certifaiger repository](https://github.com/Froleyks/certifaiger)
`main` branch at commit
`3b8d9e9937234b5e064923bd00f20d3eb97ccc3f` from 2026-07-06. It provides the
maintained witness checker and its CaDiCaL plus formally verified `lrat_isa`
path already pinned by GCC's equivalent-evidence harness. The inspected tree
does not contain the FM 2026 `aigmerge` implementation named by the paper.
The paper itself states that `aigmerge` may eventually be added to the AIGER
utilities, rather than identifying a released revision.
Therefore a direct composed-witness baseline cannot honestly be represented by
ordinary Certifaiger checking alone. The next cycle must either obtain the
authors' released implementation at an immutable revision or implement and
review a faithful paper-derived construction, clearly labelled as GCC's
baseline implementation rather than the authors' tool.

## Revised research target

The next defensible research question is narrower:

> Can a deterministic certificate encode an exact, reusable quotient of a
> separately supplied controller's input/output relation and compose it with a
> changing family of plant contracts, while a bounded low-memory checker proves
> quotient completeness, wiring integrity, and each SAFE or UNSAFE result
> without rebuilding or trusting a whole-circuit inductive witness?

This is still only a research question. To distinguish it from the closest
systems, the next comparison must use the FM 2026 composed-witness construction
or a faithful maintained implementation and predeclare these measurements:

- trust base and proof obligations;
- shared and per-plant artifact bytes;
- producer time and peak memory;
- checker time and peak memory;
- support for SAFE and UNSAFE members;
- whether changing one plant requires rebuilding the controller evidence;
- deterministic byte reproduction; and
- failure behavior for wiring, source, certificate, and resource tampering.

If GCC cannot demonstrate a semantic capability or measured property that the
composed witness approach lacks, the novelty hypothesis must be rejected. The
production case can continue on reliability, integration, and constrained
verification value without a novelty claim.

## Remaining gate

This audit materially advances but does not close novelty gate 5. Closure still
requires:

- citation chaining through the closest papers and their subsequent work;
- searches in additional scholarly and patent databases by qualified reviewers;
- claim-level analysis of relevant live patent families;
- an archived query and inclusion/exclusion log; and
- independent expert review under novelty gate 6.
