# OpenTitan PWM finite-domain predicate transducer experiment v1

Status: predeclared after three continuation-merge negative results and before
implementation. No result or novelty claim exists.

## Product question

Can GCC prove all 250 invalid runtime-channel prefixes in one exact symbolic
execution, preserving the complete input predicate and every per-input result
without replaying 250 concrete machines?

The exact-state, live-memory and live-slice experiments progressively removed
false merge barriers. The last achieved only about 1.06× to 1.09× reduction
because every invalid input still paid for its own prefix. This experiment
replaces that enumeration with a finite-domain symbolic transducer.

## Frozen scope

The pinned OpenTitan PWM source, toolchain, O0 and O2 RV32IMC images, symbols,
complete eight-bit `a0` domain, native recorder, exact GCC reference, seven
semantic classes, RTL revisions and work denominators remain unchanged.

The invalid predicate is exactly `a0 >= 6` over an unsigned eight-bit input.
It contains 250 values. Channels 0 through 5 remain explicit valid
singletons. There is no learned gate, timing calibration, sampled input or
per-image tuning.

## Exact symbolic state

The transducer represents one bounded RV32 state over the complete invalid
predicate:

- every register is an exact 250-lane `u32` function plus knownness;
- memory is one concrete base plus sparse exact 250-lane byte functions;
- the program counter and decoded instruction are shared only while every lane
  has identical control;
- loads and stores preserve exact byte width, endianness and knownness;
- branches and direct or indirect targets must be identical in all lanes;
- lane-varying addresses, targets, unsupported instructions or unresolved
  effects refuse;
- return values and every MMIO field must be constant across the predicate; and
- the full lane table remains available to reconstruct each original input.

Each admitted instruction is one symbolic transition over all lanes. The
implementation may loop over lane values internally, but must separately
report lane-value operations, wall time and peak RSS. It must not describe 250
ordinary scalar executions as one symbolic transition.

## Proof obligation

The producer emits a complete predicate transducer trace. The independent
verifier reconstructs a separate symbolic state from the image and predicate,
then checks:

1. the predicate is canonical, exhaustive for values 6 through 255 and
   disjoint from valid singleton inputs;
2. every decoded instruction and state update follows RV32IMC semantics;
3. every branch direction and target is uniform over all predicate members;
4. every sparse memory function is derived from exact stores and loads;
5. returned and observed MMIO fields are constant across the invalid class;
6. extracting any lane reproduces its exact concrete return and event stream;
7. no producer digest, representative or claimed class membership is proof;
   and
8. invalid inputs produce no RTL answer.

The valid singleton streams continue through the existing firmware-to-RTL
contract and independently checked RTL evidence.

## Candidate certificate

One deterministic certificate per profile binds:

- source, toolchain, image, symbol, predicate and policy identities;
- canonical valid singletons and the complete invalid predicate;
- every decoded symbolic transition and uniform control decision;
- exact register and sparse-memory function updates;
- final return and ordered MMIO functions;
- per-input extraction metadata;
- valid-class RTL translations, queries, revisions and answers; and
- decoded symbolic transitions, lane-value operations, allocations, wall time,
  peak RSS and artifact limits.

The decoder must bound all counts before allocation and reject noncanonical
tables, omitted or duplicated lanes, changed predicates, transition
substitution, branch or target divergence, memory aliasing errors, truncation,
extension, reordering, single-byte mutation and identity substitution.

## Static portfolio

The predicate transducer is admitted only when the complete invalid domain
follows one bounded control trace and every output is constant. Unsupported or
nonuniform source selects the complete exact reference before evidence
production. Invalid transducer evidence refuses and is never converted into
exact evidence. No partial class or RTL answer is permitted.

## Controls

Positive controls include uniform comparisons, arithmetic, sparse stack
stores, definite overwrites and constant invalid returns. Negative controls
must cover:

- one input taking a different branch;
- lane-varying load, store or indirect target addresses;
- lane-varying return, event operation, offset, value or order;
- hidden input dependence through partial stores or unknown knownness;
- a loop with lane-varying trip count;
- omitted, repeated, reordered or substituted input lanes; and
- every resource and artifact mutation class.

Each negative control must refuse before a quotient or RTL answer is visible.

## Closest work and claim boundary

Finite-domain abstract interpretation, SIMD execution, symbolic execution,
multi-execution, path predicates, KLEE, Veritesting, CBMC and explicit function
tables are established. The possible product contribution is the bounded,
proof-carrying connection from an exact binary predicate transducer through
per-input recovery to firmware-to-RTL evidence. No new symbolic-execution
algorithm is presumed.

Novelty requires focused prior-art review, identical-scope maintained angr and
CBMC comparisons and independent expert assessment.

## Predeclared gates

The mechanism passes only if:

1. all 250 extracted invalid lanes and six valid singletons agree with native
   O0/O2 and exact GCC returns and MMIO streams;
2. the independent verifier reconstructs every symbolic transition and lane;
3. O0 and O2 produce the same seven semantic classes;
4. all valid classes preserve independently checked RTL answers and invalid
   lanes produce no RTL answer;
5. every hostile control refuses without partial evidence;
6. two clean certificate cycles are byte-identical;
7. all resources and lane operations are bounded and reported;
8. producer plus verifier decode at most 157,409 symbolic or concrete
   transitions for O0 and 57,980 for O2;
9. scalar-equivalent lane operations, wall time and peak RSS are reported
   beside transition reduction and may not be hidden;
10. source-through-answer measurements include exact GCC, native, angr, CBMC
    and Veritesting baselines, including every regression;
11. hosted Linux reproduces certificate identity, answers and resources; and
12. all three earlier continuation negative controls remain negative.

Failure is retained and exact execution remains the fallback. Passing is a
bounded product result, not a universal solver or algorithmic novelty claim.
