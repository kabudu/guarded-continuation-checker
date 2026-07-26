# OpenTitan PWM guarded MMIO continuation quotient experiment v1

Status: exact all-input reference passes locally. Quotient implementation,
maintained symbolic baselines, RTL composition and hosted gates remain open.
No novelty claim exists.

## Product question

Can GCC certify every bounded runtime-input behaviour of authentic compiled
firmware without issuing one complete firmware and RTL verification job per
input?

Version 1 tests a guarded continuation quotient. It groups runtime inputs only
when independently reconstructed compiled execution proves that they have the
same return class and complete MMIO continuation. Every quotient class is then
composed with the existing source-bound RTL revision evidence.

## Frozen source and input domain

The experiment retains the OpenTitan source, toolchain, target, O0 and O2
profiles, compatibility boundary and RTL revisions from
[compiled-MMIO contract v1](OPENTITAN_PWM_COMPILED_MMIO_CONTRACT_V1.md).

The existing runtime-channel caller supplies one unconstrained eight-bit
channel value to `dif_pwm_configure_channel`. The complete domain is fixed at
all 256 values before implementation:

- channels 0 through 5 are valid OpenTitan PWM channels;
- channels 6 through 255 must return the authentic invalid-argument behaviour;
- an invalid input must not be converted into a SAFE RTL answer; and
- the input value is the only symbolic firmware input.

## Candidate certificate

One deterministic certificate per optimization profile must bind:

- the complete source, toolchain, image and symbol identities;
- the eight-bit input declaration and all 256 input values;
- a canonical, disjoint and exhaustive predicate for every quotient class;
- each class's return value and complete ordered MMIO event stream;
- a bounded decision and continuation graph over decoded RV32IMC instructions;
- every branch, indirect control, address, stored value and event dependency;
- the translation of every valid class into RTL stimulus;
- the exact RTL query and revision identities; and
- all input, graph, instruction, class, event, artifact and verification limits.

The producer may merge states only when their complete machine continuation,
including future MMIO observations and return class, is equal. Equal event
prefixes alone are insufficient.

Version 1 tests exact concrete-state convergence, not abstract or solver-assumed
state equivalence. A reusable continuation node is admitted only after byte
equality of the complete program counter, register values and knownness,
bounded memory values and knownness, and all other future execution state.
Digests may index candidate nodes but cannot establish equality. Each input
retains its own execution and event prefix before the shared node.

The independent verifier must reconstruct the binary semantics, prove class
predicates disjoint and exhaustive, replay every retained continuation edge,
and check each RTL result. It must not trust producer hashes, class membership
or representative executions as proof of unlisted inputs.

## Exact reference and maintained baselines

Before evaluating the quotient, retain:

1. 256 independent bounded GCC executions for each profile;
2. direct native recording for all 256 inputs;
3. pinned angr analysis of the identical RV32IMC images and input domain;
4. a pinned CBMC harness over the identical C caller, with explicit unwinding
   assertions and the complete eight-bit domain; and
5. the maintained Yosys, rIC3 and Certifaiger RTL evidence already used by the
   compiled-MMIO experiment.

Symbolic execution, bounded model checking and proof-carrying code are
established. The closest-work register starts with
[KLEE](https://llvm.org/pubs/2008-12-OSDI-KLEE.pdf),
[CBMC](https://diffblue.github.io/cbmc/man/cbmc.html), and
[Proof-Carrying Code](https://doi.org/10.1145/263699.263712), plus
[angr Veritesting](https://docs.angr.io/en/latest/api/angr.analyses.veritesting.html)
and
[Efficient State Merging](https://infoscience.epfl.ch/entities/publication/e52a46a1-1dd7-4b4b-ba8d-960201ab7a12).

## Hostile controls

The producer and verifier must refuse:

- missing, overlapping or non-exhaustive class predicates;
- a class containing two different return values or MMIO continuations;
- input, predicate, edge, branch-direction or representative substitution;
- a merge made from equal prefixes with different future events;
- hidden runtime influence on an address, value, jump target or observation;
- source, toolchain, image, symbol, translation, RTL or query substitution;
- certificate truncation, extension, reordering and byte mutation;
- class, graph, instruction, event or artifact limits exceeded; and
- any partial certificate or RTL result after refusal.

Negative compiled controls must include channel-dependent event order,
channel-dependent event value, divergent continuations after an equal prefix,
an indirect target influenced by the input, and a loop whose bound exceeds
policy.

## Predeclared gates

The mechanism passes only if:

1. both profiles agree with native recording, 256-run GCC reference, angr and
   CBMC for every input and return class;
2. class predicates are complete and non-overlapping over exactly 256 values;
3. every valid class preserves the independently checked RTL answer and every
   invalid class produces no RTL answer;
4. O0 and O2 yield semantically identical class partitions and MMIO streams;
5. producer and independent verifier reject the complete hostile cohort;
6. two clean builds and certificate cycles are byte-identical;
7. verifier work is bounded before graph or artifact allocation;
8. the complete quotient workflow uses at least four times fewer decoded
   firmware instruction transitions than 256 independent certificate cycles;
9. source-through-answer wall time and peak RSS are reported against every
   baseline, including any regression; and
10. hosted Linux reproduces certificate identity, answers and governed
    resource measurements.

Failure of any gate remains a retained negative result. Exact per-input
execution stays the fail-closed fallback.

For gate 8, reference work is the sum of decoded transitions for independent
certificate production and independent verification of all 256 inputs.
Quotient work is the sum of producer and independent-verifier transitions,
charging every per-input prefix and every shared continuation node exactly
once per process. The reported reduction is reference work divided by quotient
work. Setup, class construction, predicate checks and RTL composition remain
included in the wall-time and memory comparison even when they decode no
firmware instruction.

## Claim boundary

Passing would establish a bounded, proof-carrying firmware-to-RTL integration
capability. It would not by itself establish a novel algorithm. A novelty
claim requires an advantage that survives identical-scope maintained-tool
comparison, focused prior-art review and independent expert assessment.

## Exact reference result

The first implementation cycle deliberately performs no continuation reuse. A
stable concrete-`a0` RV32 API executes all 256 values independently, retains
per-input instruction work and event-producing locations, and groups only
equal return values plus complete ordered MMIO streams. Verification rebuilds
and compares the complete reference.

Pinned O0 and O2 artifacts and independently compiled native recorders agree
on every input, return value and event. Both optimization profiles produce the
same semantic partition:

| Class | Inputs | Return | Events |
| --- | ---: | ---: | ---: |
| Valid channel 0 | 1 | 0 | 16 |
| Valid channel 1 | 1 | 0 | 16 |
| Valid channel 2 | 1 | 0 | 16 |
| Valid channel 3 | 1 | 0 | 16 |
| Valid channel 4 | 1 | 0 | 16 |
| Valid channel 5 | 1 | 0 | 16 |
| Invalid channel | 250 | 2 | 10 |

O0 production decodes 314,818 instruction transitions and O2 production
decodes 115,960. Charging independent production and verification fixes the
reference-cycle denominators at 629,636 and 231,920 transitions. The quotient
must therefore remain at or below 157,409 and 57,980 respectively to pass the
predeclared fourfold gate.

Two clean, source-complete build trees are byte-identical. The retained
[arm64 result](../results/opentitan-pwm-guarded-mmio-reference-arm64-v1.txt)
binds the exact identities and work counts. This closes only the exact
reference and native-comparison prerequisites. No certificate, optimized
quotient, maintained symbolic baseline, RTL composition or novelty result
exists yet.
