# Controller MTBDD closest-system analysis

Status: primary-source scoping review, 2026-07-19. This document narrows the
next falsifiable experiment. It does not establish scholarly novelty.

## Candidate being tested

GCC produces one canonical, source-bound MTBDD for a controller's exact joint
next-state and selected-output function. A separate checker validates that
relation once, then verifies an ordered batch of independently source-bound
plant and property results. The current public-controller result shows a local
artifact and checking-cost advantage over repeating one complete verified
artifact per member.

## Closest established systems

| Established area | Capability already established | Overlap with GCC | Remaining distinction to test |
|---|---|---|---|
| CUDD ADDs and MTBDDs | Canonical decision diagrams represent finite-valued functions and support shared manipulation | The controller relation is exactly an MTBDD-style finite-valued function | Source binding, hostile-input checking, and a portable multi-environment result bundle are integration properties, not a new decision diagram |
| BDD symbolic model checking | BDDs and partitioned transition relations share structure across symbolic reachability operations | Compiling a transition relation once and checking many properties is standard | GCC must compare against one compiled model-checker session, not only repeated standalone artifacts |
| Assume-guarantee verification | Component assumptions characterise environments in which a component satisfies a property and enable compositional reuse | GCC similarly separates a controller from supplied environments | GCC currently verifies concrete environment members, not a weakest reusable assumption, so it cannot claim stronger compositional reasoning |
| ABC sequential verification | AIG rewriting, SAT reasoning, model checking, and equivalence checking are combined in an established hardware flow | GCC consumes AIGER and checks a functional relation back to its source model | The canonical offline artifact and smaller checker boundary may be useful, but require a direct trust and cost comparison |
| Certifaiger and k-witness circuits | Independently checkable certificates establish hardware safety results with compact witness circuits | Both systems move trust from a producer to a checker | Certifaiger certifies a safety result; GCC's candidate distinction is one checked controller function reused across several separately bound results |
| Proof-carrying hardware via IC3 | Automatically produced sequential safety evidence can be much cheaper to validate than to generate | This already establishes the general proof-carrying hardware value proposition | GCC must show a distinct reusable evidence contract rather than claim proof carrying itself |

## Primary sources

- Burch, Clarke, and Long, [Symbolic Model Checking with Partitioned Transition Relations](https://kilthub.cmu.edu/articles/journal_contribution/Symbolic_Model_Checking_with_Partitioned_Transition_Relations/6610106).
- Somenzi, [CUDD: CU Decision Diagram Package](https://www.cs.uleth.ca/~rice/cudd_docs/).
- Brayton and Mishchenko, [ABC: An Academic Industrial-Strength Verification Tool](https://people.eecs.berkeley.edu/~alanmi/publications/2010/cav10_abc.pdf).
- Giannakopoulou, Păsăreanu, and Barringer, [Component Verification with Automatically Generated Assumptions](https://doi.org/10.1007/s10515-005-2641-y).
- Yu, Biere, and Heljanko, [Progress in Certifying Hardware Model Checking Results](https://fmv.jku.at/papers/YuBiereHeljanko-CAV21.pdf), and the [Certifaiger project](https://fmv.jku.at/certifaiger/).
- Isenberg and Wehrheim, [Proof-Carrying Hardware via IC3](https://arxiv.org/abs/1410.4507).

## Finding

The current MTBDD representation and compile-once reuse are not novel. The
canonical source binding, independently checked relation, and ordered batch
binding may form a useful evidence-delivery contract, but the present repeated
complete-artifact baseline does not isolate that value from ordinary in-process
model compilation and reuse.

The next experiment must therefore use a representative physical plant family
and compare three predeclared paths:

1. one GCC shared artifact checked by a fresh consumer;
2. one ordinary compiled symbolic model reused in-process for the same ordered
   queries; and
3. one independently complete artifact per query.

Every path must receive identical controller, initial state, wiring, monitors,
and horizons. Report setup, query, serialization, checking, and total time
separately, plus peak memory and bytes crossing the producer-consumer boundary.
The ordinary compiled-model path is the runtime baseline. The repeated complete
artifact path is only the evidence-transfer baseline.

## Advancement gate

Advance this candidate only if the shared artifact:

- preserves exact SAFE and UNSAFE answers and shortest bad frames;
- is checked without trusting producer-side caches or object identity;
- beats repeated complete evidence in bytes and consumer checking time; and
- demonstrates a concrete trust-transfer or deployment advantage that the
  faster ordinary in-process path does not provide.

No claim is allowed merely because GCC beats the repeated-artifact baseline.
If the ordinary model checker can export an equivalently bound and independently
checkable batch at comparable cost, this candidate distinction is falsified.

The first [whole-process comparison](CONTROLLER_MTBDD_PROCESS_RESOURCES_V1.md)
is negative on speed: the maintained SymbiYosys route is 2.67 times faster than
fresh GCC verification on the retained arm64 host. GCC uses 85.2% less peak RSS
and transfers an 8,549-byte independently checked artifact, while the oracle
must rerun and emits no comparable replay artifact. This demonstrates the
predeclared evidence-delivery distinction but does not establish novelty.

The first [MTBDD equivalence-proof experiment](CONTROLLER_MTBDD_EQUIVALENCE_PROOF_V1.md)
targets that negative result. A SAT miter and independently checked UNSAT proof
reduce in-process equivalence checking from 875.03 ms to 7.20 ms on one retained
run, while adding 242,496 raw proof bytes. SAT-based circuit equivalence and
proof logging are established, so the improvement is an engineering tradeoff,
not a new verification principle. Whole-process hosted comparison against the
maintained formal route is required before admitting it to the portfolio.

The follow-up
[identical-query maintained-tool baseline](CONTROLLER_PROOF_MTBDD_MAINTAINED_BASELINE_V1.md)
is negative on runtime. The maintained SymbiYosys/Yosys/Z3 oracle is 1.33 times
faster than a fresh proof verifier and about 5.39 times faster than initial proof
creation plus verification. Proof verification uses 65.5% less peak RSS and
supplies portable evidence, but proof-carrying hardware already establishes that
general value. This closes the local maintained-tool runtime question without
supporting a novelty claim.
