# Dense controller relation v1 experiment

## Status

This document predeclares the next experiment after controller-obligation reuse.
The first extraction step is now implemented: a public, resource-bounded dense
relation module supplies exact edge insertion, membership, target enumeration,
composition, and binary exponentiation. The existing predicate engine uses that
module without changing its certificate format or answers. This remains an
experiment and falsification plan, not a novelty or production-readiness claim.

The braking obligation proves that one independently checked controller summary
can reduce artifact size and checking time across several plants. Its vocabulary
is too narrow for unmodified public embedded controllers. Dense controller
relation v1 tests whether the existing predicate v2 proof primitives can become
a general reusable component boundary.

## Candidate capability

For one source-bound AIGER or BTOR2 controller, extract a finite relation over:

- selected controller latch bits before a step;
- selected environment input bits;
- selected controller latch bits after the step; and
- selected controller output bits exposed to the environment.

Each admitted edge must carry a concrete input witness. Each source row must
carry an independently checked UNSAT proof that no omitted target is possible.
The obligation therefore proves both relation soundness and completeness. It is
not a sampled transition table.

Several plant, bus, scheduler, or fault-environment members may then compose
against the same controller relation. The final verifier must check the
controller obligation once, check every environment member independently, and
reconstruct SAFE or UNSAFE results and traces. Any unsupported shape, resource
limit, proof failure, or composition intersection retains the exact original
portfolio query.

## Reuse boundary

Predicate certificate v2 already implements the required local proof primitives:

- exact edge witnesses;
- per-row CNF completeness obligations;
- Varisat-native UNSAT proofs;
- an independent AIGER evaluator and proof checker;
- canonical bounded artifacts; and
- exact portfolio fallback.

The experiment must extract these primitives into a stable library module rather
than copy private executable code. The old predicate v2 artifact remains valid.
Refactoring must preserve its frozen compatibility fingerprints and command-line
contract.

## Static resource gate

The first version is bounded to:

- at most 16 selected environment inputs;
- at most 4 selected controller latches;
- at most 4 selected controller outputs;
- at most 16 relation states;
- at most 65,536 witnessed edges;
- at most 1 MiB per UNSAT proof;
- at most 8 MiB total proof bytes; and
- at most 64 environment members.

These are admission limits, not approximations. Exceeding any limit routes the
unchanged query to exact fallback.

## Predeclared baselines and gates

| Gate | Required result |
|---|---|
| Exactness | Every composed answer and trace agrees with ordinary predicate v2 and exact CDCL |
| Independent proof | A checker that does not call the producer validates every edge witness and row-completeness proof |
| True reuse | Controller parsing, relation validation, and proof checking occur once per batch |
| Strong artifact baseline | Shared relation plus members is smaller than straightforward ordinary proof-carrying certificates |
| Strong checking baseline | In-process checking beats a parse-once shared-model ordinary baseline |
| Production cost | A reported verification count amortises extraction, proof generation, and checking |
| Static selection | Routing uses source structure and declared bounds only, never trial timing |
| Hostile input | Source, boundary, proof, member, order, count, truncation, and size mutations fail closed |
| Maintained tools | Yosys plus at least one maintained hardware model checker agrees on every retained public case |
| Public product | One revision-pinned, unmodified embedded RTL controller family supplies at least two environment or parameter members |
| Cross-platform | Library production and checking agree on Linux, macOS, and Windows |

Failure of the strong artifact or checking baseline falsifies the reuse-efficiency
hypothesis for that cohort. Fixture-only success is insufficient.

## Public product gate

Established BTOR2 collections include real open-source hardware derived from
projects such as ZipCPU, PicoRV32, PonyLink, and riscv-formal. Those collections
are primarily monolithic verification tasks. They cannot honestly be relabelled
as pre-separated controller and plant models.

The experiment must therefore preserve the upstream controller source unchanged
and add only provenance-recorded verification wrappers or environment modules.
The first preferred cohort is a small revision-pinned processor control block
with at least two bus, interrupt, or fault environments. Every upstream file,
license, revision, and SHA-256 digest must be retained. Repository-authored
wrappers must be identified separately and may not be described as upstream
product code.

## Closest prior art and claim boundary

Finite transition relations, BDD relational products, assume-guarantee
reasoning, proof-carrying model checking, witness circuits, and reusable
component assumptions are established. Relevant boundaries include:

- Btor2, BtorMC and Boolector 3.0, which introduced the word-level format,
  parser, simulator, witness checker, and real open-source hardware benchmarks;
- Btor2-Cert, which creates independently checkable hardware-verification
  evidence through software analyzers;
- certificates for the Hardware Model Checking Competition;
- compositional synthesis of modular systems; and
- learning assumptions for compositional verification.

No individual data structure or proof primitive above is claimed as novel. A
candidate claim would require measured evidence that a source-bound, complete,
proof-carrying controller relation is reused across unmodified embedded-product
environments and beats strong ordinary proof-carrying composition baselines.
That claim is currently unproven.

## Implementation sequence

1. Extract the relation, witness, CNF obligation, and independent proof-checking
   primitives from the executable into stable library modules. The relation
   primitive is complete; witness and proof primitives remain private.
2. Prove byte-for-byte and answer compatibility for predicate certificate v2.
3. Add a source-bound controller boundary manifest and canonical obligation.
4. Add environment-member composition with exact trace reconstruction.
5. Add a static portfolio with unchanged exact fallback.
6. Run hostile-input and three-platform gates.
7. Import one revision-pinned public embedded controller family with explicit
   provenance and maintained-tool agreement.
8. Run the strong artifact, production, checking, and amortisation benchmarks.
