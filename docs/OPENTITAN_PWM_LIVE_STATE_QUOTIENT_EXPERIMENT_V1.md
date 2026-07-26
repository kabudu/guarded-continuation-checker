# OpenTitan PWM proof-carrying live-state quotient experiment v1

Status: predeclared after the exact-state negative result and before
implementation. No result or novelty claim exists.

## Product question

Can GCC reuse a compiled firmware continuation when concrete states differ
only in bytes that an independently checked suffix proof establishes cannot
affect any future instruction, return value or MMIO observation?

The preceding exact-state experiment refused both authentic optimization
profiles because inputs 6 and 7 leave different stale stack bytes in bounded
memory. This experiment does not treat equal terminal behaviour as proof.
Instead, it adds an explicit live-in memory obligation to exact continuation
equality.

## Frozen scope

The source, toolchain, RV32IMC images, symbols, O0 and O2 profiles, complete
eight-bit `a0` domain, native reference, RTL revisions and exact-reference
denominators remain unchanged from
[guarded MMIO quotient v1](OPENTITAN_PWM_GUARDED_MMIO_QUOTIENT_EXPERIMENT_V1.md).
There is no per-formula, per-image or timing calibration.

Channels 0 through 5 remain singleton valid classes. Inputs 6 through 255 are
eligible for one invalid-input class only if every member passes the live-state
proof. Any unsupported instruction, unresolved address, hidden runtime
influence, policy refusal or failed proof selects the complete exact reference
before an RTL answer is produced.

## Live-state invariant

At a candidate merge, the following must be byte-equal:

- program counter, execution-bound state and previous control location;
- every register value and knownness bit;
- event-count state and the complete retained event prefix;
- every memory byte and knownness bit in the certified live-in set; and
- all source, image, symbol and policy identities.

The live-in set is reconstructed from the representative continuation by a
backward read-before-overwrite analysis over the exact decoded suffix:

1. every byte read before a definite overwrite is live;
2. a definite store kills only the exact bytes it overwrites;
3. partial and overlapping accesses are handled byte by byte;
4. instruction bytes, event state, observable MMIO state and return
   dependencies cannot be declared dead;
5. unknown or unsupported addresses, control, widths or instructions refuse;
6. loops require a complete bounded suffix, not a sampled iteration; and
7. a differing byte outside the live-in set is admissible only when replay
   proves that it is overwritten before any read or never read.

The verifier must replay every candidate prefix to its merge state,
reconstruct the representative live-in set without invoking the producer,
compare all required state, and replay the shared suffix once. An
instruction-by-instruction induction must establish that equal registers and
equal live-in bytes preserve every later load, store, branch, target, return
and MMIO event. Producer digests may locate candidates but are never proof.

## Candidate certificate

One canonical certificate per profile must bind:

- source, toolchain, image, symbol and policy identities;
- all 256 input values and a disjoint, exhaustive class partition;
- every per-input prefix and the exact merge state identifier;
- the complete decoded representative suffix;
- byte-precise suffix reads, writes and backward live-in sets;
- all differing bytes excluded from equality and their deadness reason;
- return values, ordered MMIO streams and event-producing locations;
- valid-class RTL translations, queries, revisions and answers;
- producer and verifier decoded-transition accounting; and
- input, instruction, prefix, suffix, memory, live-byte, event, certificate,
  verification-work, wall-time and resident-memory limits.

The encoding must be deterministic, bounded before allocation, canonical and
covered by truncation, extension, reordering and single-byte mutation tests.

## Exact fallback

Routing is static:

- use the quotient only after all certificate and live-state obligations pass;
- use the complete 256-input exact reference when the candidate is unsupported
  or a live-state equality cannot be established; and
- reject invalid quotient evidence without retrying it as exact evidence.

No partial quotient, partial RTL answer or mixed route is permitted.

## Hostile controls

The producer and verifier must reject:

- a differing stack byte read after the proposed merge;
- a differing byte used in a later load address, store address, stored value,
  branch condition, indirect target, return or MMIO event;
- a byte marked dead before a partial or aliased read;
- a store claimed to kill more bytes than it overwrites;
- a path, loop iteration, read, write or event omitted from the suffix;
- candidate and representative states with unequal registers, knownness,
  event prefixes or live memory;
- class overlap, omission, input substitution or route substitution;
- source, toolchain, image, symbols, RTL, query or policy substitution;
- resource-limit overflow before or during reconstruction; and
- every certificate truncation, extension, reordering and byte mutation.

A positive stale-stack control must differ only in bytes proven dead. A
near-neighbour must read one such byte after the merge and be refused.

## Closest work and claim boundary

This experiment combines established ideas:

- LLVM
  [MemorySSA](https://llvm.org/docs/MemorySSA.html) and its byte-affecting
  dead-store reasoning;
- CompCert's verified
  [liveness analysis](https://compcert.org/doc/html/compcert.backend.Liveness.html)
  and semantic-preservation framework;
- KLEE, Veritesting and efficient symbolic-state merging from the preceding
  register;
- proof-carrying code and translation validation; and
- observational equivalence, dynamic slicing and state subsumption.

The possible contribution is the bounded combination of binary-reconstructed
live memory, proof-carrying continuation reuse, exact per-input recovery and
firmware-to-RTL evidence. That combination is not presumed novel. A novelty
claim requires a focused literature review, identical-scope maintained-tool
baselines and independent expert assessment.

## Predeclared gates

The mechanism passes only if:

1. every input agrees with native O0/O2 recording and the exact GCC reference;
2. the independent verifier reconstructs every live-in set and class without
   trusting producer membership or hashes;
3. every valid class retains its independently checked RTL answer and invalid
   classes produce no RTL answer;
4. O0 and O2 retain the same semantic partition and MMIO streams;
5. the complete hostile cohort refuses without partial evidence;
6. two clean certificate cycles are byte-identical;
7. all allocations and producer/verifier work are preflight bounded;
8. producer plus verifier decode at most 157,409 transitions for O0 and 57,980
   for O2, preserving the preceding fourfold gate;
9. source-through-answer wall time and peak RSS are reported against the exact
   reference, native, angr, CBMC and Veritesting baselines, including
   regressions;
10. hosted Linux reproduces certificate identity, answers and resource
    measurements; and
11. the exact-state negative control still refuses while the certified-dead
    stale-stack control passes.

Failure of any gate is retained as a negative result. Passing establishes a
bounded product capability, not by itself a novel algorithm or a universal
firmware result.
