# OpenTitan PWM multi-successor decode graph v2

## Question

Can GCC share decoded firmware instructions across paths that temporarily
reconverge while preserving the complete hidden machine state needed to select
and validate the path that each input actually takes?

Version 1 answered a narrower question negatively. Its single-successor nodes
encoded the entire future control suffix, so different return addresses kept
otherwise common code separate. Version 2 does not intern suffix state. It
shares only source-bound decoded instructions and independently replays all
machine state.

## Mechanism

The producer executes every input from 0 through 255. It constructs the exact
union of observed control-flow transitions as a canonical directed graph.

Each node binds:

- program counter;
- instruction word and width fetched from the firmware image; and
- a sorted set of observed next program counters.

The artifact also binds the exact firmware image digest, symbol layout,
complete per-input terminal behavior and total scalar work. Nodes are ordered
canonically by program counter, instruction word and width. Edges are strictly
increasing and duplicate-free.

The consumer receives canonical bytes, the firmware image and symbol layout.
For every input it creates a fresh scalar replay machine. At each step it:

1. locates the graph node for the machine's current program counter;
2. fetches and checks the instruction bytes directly from the bound image;
3. executes one scalar machine transition;
4. checks that the resulting program counter is an edge of that node; and
5. continues until it independently checks the declared terminal behavior.

The graph never selects a branch and contains no abstract machine-state
summary. The replayed register, memory, call and control state remains the
authority that selects the next program counter. The graph only certifies that
the exact source instruction and observed transition belong to the complete
finite-domain execution union.

Missing and additional nodes or edges, duplicate encodings, unreachable
content, uncovered content, source drift, symbol drift, terminal drift,
truncation, extension and checksum changes refuse.

## Frozen cohort

The cohort is identical to version 1:

- pinned OpenTitan PWM DIF and compatibility boundary;
- O2 RV32IMC compilation;
- controlled dispatcher class counts 1, 2, 4, 8, 16 and 32;
- all 256 byte-valued inputs; and
- unchanged valid-channel and invalid-channel behavior.

The dispatcher remains benchmark harness code rather than upstream OpenTitan
production firmware. The cohort, compiler profile, domain and class counts are
frozen before the first complete version 2 sweep.

## Baselines

Every cohort member is compared with:

1. the faithful explicit per-input execution transcript;
2. the canonical non-sharing trace-family certificate;
3. the rejected single-successor continuation DAG; and
4. the multi-successor decode graph.

All four routes must bind the same firmware image and recover byte-identical
complete input-to-MMIO semantics. The trace family is the primary closest
baseline because it already removes duplicate complete traces without sharing
temporarily common instructions.

## Measurements

For every class count, retain:

- canonical artifact bytes for all four representations;
- graph nodes, graph edges and trace-family decoded transitions;
- producer scalar steps and verifier scalar steps;
- unique verifier instruction decodes;
- five warm-consumer wall-time and peak-RSS trials;
- normalized complete semantic identity; and
- hostile refusal counts by mutation category.

Two clean production cycles must produce byte-identical graph artifacts and
normalized results.

## Predeclared gates

The version 2 mechanism succeeds only if:

1. every route recovers byte-identical complete semantics for all 256 inputs;
2. two clean cycles produce byte-identical artifacts at all six class counts;
3. every retained mutation, truncation, extension, node, edge, terminal,
   image and symbol change refuses;
4. every graph node and edge is exercised by at least one independent scalar
   replay, with no undeclared transition accepted;
5. graph artifact size and node count are monotonic nondecreasing as declared
   control classes increase;
6. at eight classes, the graph is no more than 75 percent of the canonical
   trace-family bytes;
7. at eight classes, unique graph instruction decodes are no more than
   75 percent of trace-family decoded transitions;
8. at 32 classes, the graph remains smaller than both the trace family and the
   single-successor DAG;
9. at every admitted class count, graph median warm-consumer time is no more
   than 1.25 times the trace-family median, and graph maximum RSS is no more
   than twice the trace-family minimum; and
10. all producer work, scalar replay work, graph lookup work and losing
    regimes remain visible.

No class count, threshold, compiler flag, route, trial count, artifact field
or timing definition may change after the first complete version 2 sweep is
observed.

## Claim boundary

Passing would establish a bounded representation result: on the controlled
OpenTitan PWM cohort, one source-bound multi-successor decode graph can share
temporarily reconvergent instructions across exact finite-domain paths while
fresh scalar replay preserves and checks the hidden state that selects each
outgoing edge.

It would not establish arbitrary firmware support, universal compression,
solver acceleration, a safety proof independent of bounded execution,
production readiness or superiority over established control-flow graph and
trace-compression systems. A passing controlled result would require
qualification on unrelated public firmware before any broader claim.

## Local result

Two clean sweeps are byte-identical across every firmware binary, symbol
table, artifact and normalized result. All routes recover identical complete
semantics for all 256 inputs.

| Classes | Graph bytes | Trace-family bytes | Size ratio | Graph decodes | Trace-family decodes | Decode ratio |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 7,872 | 17,413 | 45.21% | 316 | 1,113 | 28.39% |
| 2 | 8,759 | 23,332 | 37.54% | 367 | 1,568 | 23.41% |
| 4 | 10,726 | 35,476 | 30.23% | 470 | 2,488 | 18.89% |
| 8 | 13,979 | 59,314 | 23.57% | 667 | 4,334 | 15.39% |
| 16 | 20,803 | 107,082 | 19.43% | 1,059 | 8,006 | 13.23% |
| 32 | 34,451 | 202,618 | 17.00% | 1,843 | 15,350 | 12.01% |

At 32 classes, the rejected single-successor DAG is 264,398 bytes. The
multi-successor graph is 34,451 bytes. Graph nodes and bytes increase
monotonically across the frozen cohort.

The hostile matrix refuses 193,228 cases. These cover every single-byte
mutation, every truncation, extensions, firmware and symbol substitutions,
missing and additional edges, duplicate edges, missing nodes and terminal
drift across all six artifacts.

## Timing falsification

The five-trial warm-consumer resource cycle fails the frozen time limit at two
class counts and the RSS limit at one class count:

| Classes | Graph median | Trace-family median | Graph maximum RSS | Trace-family minimum RSS | Time | RSS |
| ---: | ---: | ---: | ---: | ---: | :--- | :--- |
| 1 | 0.01 s | 0.01 s | 4,177,920 | 4,177,920 | pass | pass |
| 2 | 0.01 s | 0.01 s | 10,485,760 | 4,259,840 | pass | fail |
| 4 | 0.01 s | 0.01 s | 6,406,144 | 6,438,912 | pass | pass |
| 8 | 0.01 s | 0.01 s | 4,358,144 | 6,586,368 | pass | pass |
| 16 | 0.02 s | 0.01 s | 8,699,904 | 6,914,048 | fail | pass |
| 32 | 0.02 s | 0.01 s | 8,962,048 | 7,569,408 | fail | pass |

The process durations are near the local timer resolution, but the
predeclared gate does not permit recalibration or omission. Version 2 is
therefore not fully admitted.

The implementation performs a balanced-tree program-counter lookup on every
scalar replay step. The closest trace-family consumer advances directly
through a vector. A separately predeclared successor may test a dense
source-offset index while preserving the same graph bytes, scalar semantics
and refusal boundary.
