# OpenTitan PWM branching continuation DAG v1

## Question

Can GCC retain its independently checked representation advantage when a
complete finite firmware-input domain follows several different control-flow
paths rather than one uniform invalid path?

The experiment generalizes the retained predicate certificate into a
canonical continuation DAG. It does not change the bounded RV32IMC semantics,
input domain, caller-owned firmware image or independent scalar replay
boundary.

## Mechanism

The producer executes every input from 0 through 255 and records its exact
ordered control steps and terminal MMIO behavior. It then constructs a
canonical DAG by interning equal continuation suffixes from terminal state
backward. Each input has one root node. Equal suffixes share one node
regardless of which earlier branch reached them.

Every node binds:

- program counter;
- decoded instruction word and width;
- next program counter;
- next-node identity; and
- terminal behavior identity where the path ends.

The consumer receives canonical bytes, the exact firmware image and symbol
layout. For every input it creates a fresh scalar replay machine, fetches and
checks its own instruction bytes, follows the declared root through the DAG,
replays every state transition and compares the terminal return value, MMIO
events and event program locations. Cycles, missing roots, unreachable nodes,
noncanonical node order, source drift and incomplete paths refuse.

This is a proof-carrying execution representation. The producer's exhaustive
execution is not trusted by the consumer.

## Frozen cohort

The cohort uses the pinned OpenTitan PWM DIF and compatibility boundary with
O2 RV32IMC compilation. A controlled dispatcher around the authentic DIF call
creates 1, 2, 4, 8, 16 and 32 invalid-domain control classes. Each class uses a
distinct noinline call path before reaching the same authentic invalid-channel
check. Inputs 0 through 5 retain the six valid channel behaviors.

The class counts, source template, toolchain, optimization profile and input
domain are frozen before the first complete sweep. The dispatcher is benchmark
harness code, not represented as upstream OpenTitan production firmware.

## Baselines

Each cohort member is compared with:

1. the faithful explicit execution transcript introduced by the closest-system
   experiment;
2. a non-sharing trace-family certificate containing one complete trace per
   control class; and
3. the canonical continuation DAG.

All routes bind the same image and recover the same complete input-to-MMIO
semantics. The explicit transcript remains the primary closest-system
baseline. The trace family isolates the value of suffix interning from the
value of input partitioning.

## Measurements

For every class count, retain:

- canonical artifact bytes;
- unique DAG nodes and total scalar path steps;
- producer and verifier decoded transitions;
- scalar replay steps;
- producer wall time;
- five warm-consumer wall-time and peak-RSS trials;
- normalized complete semantic identity; and
- hostile refusal counts.

Two clean production cycles must be byte-identical.

## Predeclared gates

The branching mechanism succeeds only if:

1. every route recovers byte-identical complete semantics for all 256 inputs;
2. two clean cycles produce byte-identical artifacts for every class count;
3. every retained hostile mutation, truncation, extension, root, edge,
   terminal, image and symbol change refuses;
4. the DAG has no unreachable node and independently replays every input;
5. artifact size is monotonic nondecreasing as declared control classes
   increase;
6. at eight classes, the DAG is at least 10 times smaller than the explicit
   transcript;
7. at eight classes, DAG verifier decoded transitions are at least 3 times
   fewer than the explicit transcript verifier transitions;
8. at every admitted class count, DAG median warm-consumer time is no greater
   than the explicit transcript median and DAG maximum RSS is no greater than
   twice the explicit transcript minimum; and
9. all producer work, scalar replay work and losing regimes remain visible.

No class count, threshold, compiler flag, route, trial count or artifact
definition may change after the first complete sweep result is observed.

## Claim boundary

Passing would establish a bounded generalization: GCC can preserve a measured
representation advantage over an explicit proof-carrying execution transcript
across several exact firmware control-flow classes by sharing certified
continuations.

It would not establish universal compression, arbitrary firmware support,
algorithmic novelty, production readiness or superiority over all software and
hardware witness systems. The cohort deliberately controls branch structure
and must be followed by unrelated public firmware if it passes.

## Local result

Two clean sweeps produce byte-identical complete semantics and artifacts at
all six class counts. The DAG remains substantially smaller than the explicit
per-input transcript:

| Classes | DAG bytes | Explicit bytes | Size reduction | Decode reduction |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 22,369 | 2,455,008 | 109.75 times | 104.87 times |
| 2 | 30,104 | 2,464,508 | 81.87 times | 74.76 times |
| 4 | 45,920 | 2,481,114 | 54.03 times | 47.47 times |
| 8 | 77,126 | 2,494,258 | 32.34 times | 27.39 times |
| 16 | 139,550 | 2,494,258 | 17.87 times | 14.83 times |
| 32 | 264,398 | 2,494,258 | 9.43 times | 7.73 times |

The exhaustive hostile matrix refuses all 1,158,952 tested DAG mutations,
truncations, extensions, firmware substitutions and symbol substitutions.

## Closest-baseline falsification

The canonical non-sharing trace-family codec changes the conclusion. It
contains one full trace per distinct control path and uses the same image
binding, terminal evidence and independent scalar replay:

| Classes | DAG nodes | Trace-family decoded transitions | DAG bytes | Trace-family bytes |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 1,113 | 1,113 | 22,369 | 17,413 |
| 2 | 1,568 | 1,568 | 30,104 | 23,332 |
| 4 | 2,488 | 2,488 | 45,920 | 35,476 |
| 8 | 4,334 | 4,334 | 77,126 | 59,314 |
| 16 | 8,006 | 8,006 | 139,550 | 107,082 |
| 32 | 15,350 | 15,350 | 264,398 | 202,618 |

No continuation suffix is shared beyond complete-trace grouping. Different
wrapper return addresses keep the future control suffixes distinct even while
the paths temporarily execute common code. The DAG therefore performs no
fewer unique decodes and is about 30 percent larger because each node carries
an edge field.

The local resource cycle also fails the strict no-slower gate at two classes:
the DAG median is 0.03 seconds versus 0.02 seconds for the explicit consumer.
These process durations are near timer resolution, but the frozen gate does
not permit recalibration or discarding the result.

## Conclusion

Version 1 fails gates 6 through 8 as a continuation-sharing mechanism and is
not admitted. It does establish a useful negative boundary: grouping inputs
by identical complete control traces creates the large representation and
decode advantage over per-input transcripts, while a single-successor suffix
DAG adds no benefit for temporarily reconvergent code with different hidden
return state.

The appropriate successor is a source-bound multi-successor decode graph.
Such a graph may share one decoded instruction at a common program counter
while the independently replayed machine state selects and validates the
actual next edge. That is a different mechanism and requires a new
predeclaration.
