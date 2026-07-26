# OpenTitan PWM proof-carrying live-slice quotient experiment v1

Status: predeclared after the live-memory regression and before implementation.
No result or novelty claim exists.

## Product question

Can GCC certify the earliest reusable compiled continuation by comparing only
the registers and memory bytes that a complete, independently reconstructed
suffix can still observe?

The exact-state candidate failed because stale stack bytes differed. The
live-memory candidate proved those bytes dead, but required all registers to
remain equal and therefore merged only near return. This experiment adds
register liveness and replaces repeated suffix probing with one backward
live-slice reconstruction.

## Frozen scope and routing

The authentic OpenTitan PWM source, pinned toolchain, RV32IMC images, O0 and O2
profiles, all 256 `a0` values, native reference, exact GCC reference, RTL
revisions and work denominators remain unchanged. The fourfold ceilings remain
157,409 transitions for O0 and 57,980 for O2, charging producer and verifier.

Channels 0 through 5 remain valid singleton classes. Inputs 6 through 255 may
share one invalid continuation only after every member passes the live-slice
proof. There is no timing, per-image or per-formula calibration. Unsupported
semantics or an unproved slice select the complete exact reference. Invalid
quotient evidence refuses and is never retried as exact evidence.

## Live-slice invariant

The representative is executed once to completion while retaining, for every
decoded instruction:

- program counter and decoded instruction identity;
- registers read and definitely written;
- exact memory bytes read and definitely overwritten;
- branch direction, direct or indirect target and address dependencies;
- event-producing locations and ordered MMIO effects; and
- cumulative decoded-transition work.

The checker reconstructs liveness backwards:

1. `a0`, final event memory and every other returned or observed value are live
   at completion;
2. instruction reads add registers or bytes to the live set;
3. a definite write kills only its exact destination register or bytes;
4. partial and overlapping memory operations are processed byte by byte;
5. instruction fetch bytes and code identity remain observable;
6. unknown addresses, targets, widths, effects or unsupported instructions
   refuse;
7. every bounded loop iteration remains in the suffix; and
8. a dead register or byte may differ only when the reconstructed suffix
   neither reads it before overwrite nor exposes it as an observation.

At a merge, the program counter, execution-bound state, event prefix and every
live register value and knownness bit and live memory value and knownness bit
must be equal. Dead state may differ. An instruction-by-instruction induction
must show that equality of the live slice yields equal next live slices,
control, return values and MMIO observations.

## Candidate selection

The producer must not replay a suffix for each possible merge:

1. execute one invalid representative completely and reconstruct the live
   slice at every suffix position;
2. replay inputs 6 and 7 in lockstep and select the earliest position whose
   control state and reconstructed live slice are equal;
3. replay every other invalid input only to that fixed position;
4. compare it with the same independently reconstructed slice; and
5. reuse the already executed representative suffix exactly once.

The verifier independently repeats this algorithm without calling the
producer. Candidate discovery, liveness reconstruction, rejected positions,
all prefixes and the one shared suffix count as work.

## Certificate and hostile controls

One deterministic certificate per profile must bind all source and binary
identities, the complete input partition, merge position, per-input prefix,
decoded representative suffix, register and memory use/def facts, live slices,
excluded differences, return and MMIO behaviours, RTL translations and
answers, resource policy and exact producer/verifier work.

The canonical bounded decoder must reject truncation, extension, reordering,
single-byte mutation, overlapping or incomplete classes, changed merge
positions, forged use/def facts, omitted instructions, dead `a0`, dead branch
or address inputs, partial-store overclaims, aliasing, a read of excluded state,
source or binary substitution, and every resource overflow before allocation.

Positive controls must cover dead registers, dead stack bytes, definite
overwrite before read and partial stores. Near-neighbours must turn each into a
later return, branch, address, indirect target or MMIO dependency and refuse.

## Closest work and claim boundary

Register liveness, memory liveness, backward slicing, dead-store elimination,
symbolic state subsumption and observational equivalence are established. The
closest-work register includes LLVM
[MemorySSA](https://llvm.org/docs/MemorySSA.html), CompCert
[liveness](https://compcert.org/doc/html/compcert.backend.Liveness.html), KLEE,
Veritesting, efficient state merging, proof-carrying code and translation
validation.

The research question is the bounded combination of independently checkable
binary live slices, continuation reuse, exact input recovery and firmware-to-RTL
evidence. Passing does not establish a new liveness algorithm. Any novelty
claim requires maintained-tool comparisons, focused literature review and
independent expert assessment.

## Predeclared gates

The mechanism passes only if:

1. all 256 answers and MMIO streams agree with native O0/O2 and exact GCC;
2. the verifier independently reconstructs every use, def, slice and route;
3. O0 and O2 preserve the same seven semantic classes;
4. valid classes retain independently checked RTL answers and the invalid
   class produces no RTL answer;
5. every hostile and near-neighbour control refuses without partial evidence;
6. two clean certificates and verification cycles are byte-identical;
7. resource limits preflight all trace, slice, prefix and artifact allocation;
8. total decoded transitions do not exceed 157,409 for O0 or 57,980 for O2;
9. wall time and peak RSS include all setup and are reported against exact GCC,
   native, angr, CBMC and Veritesting, including regressions;
10. hosted Linux reproduces identities, answers and governed measurements; and
11. both earlier negative controls remain negative.

Failure remains a retained negative result and exact execution remains the
fallback. Passing is a bounded product result, not a universal firmware or
algorithmic novelty claim.
