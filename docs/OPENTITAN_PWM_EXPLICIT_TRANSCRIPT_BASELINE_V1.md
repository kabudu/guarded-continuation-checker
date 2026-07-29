# OpenTitan PWM explicit execution-transcript baseline v1

## Purpose

This experiment tries to falsify the narrowest remaining GCC distinction.
It compares GCC's compiled-binary/MMIO predicate certificate with a
straightforward proof-carrying execution transcript at the same finite-domain
firmware boundary. The baseline is deliberately strong: its consumer receives
the frozen firmware image and symbol layout, initializes a fresh bounded
RV32IMC machine for every input, replays every instruction, and compares every
certified observation. It never trusts a producer-supplied result or skips
source execution.

The RTL replay is common to both routes and is therefore held constant. The
first gate isolates the firmware representation before composing the winning
route with the existing six-channel OpenTitan PWM RTL certificate.

## Prior-art boundary

The broad space is established prior art:

| System | Relevant capability | Included in this experiment | Excluded distinction |
| --- | --- | --- | --- |
| Proof-Carrying Hardware via IC3 | Independently checked sequential-hardware certificates | Independent consumer and source-bound result | Not a firmware/MMIO finite-domain transcript comparison |
| Certifaiger and k-witnesses | Compact independently checkable hardware safety witnesses | Canonical bounded artifact and hostile checking | Hardware witness rather than compiled firmware behavior |
| Btor2-Cert | Transports software-verification witnesses into BTOR2 witnesses | Cross-representation certificate transport | Does not establish a GCC-specific representation advantage |
| SV-COMP witnesses | Interoperable witnesses with independent validators | Consumer-side validation principle | Software witnesses rather than the composed firmware/RTL boundary |
| HW-CBMC and VerifOx | Joint C and Verilog reasoning through a solver | Same-answer hardware/software comparison | Regenerates a monolithic formula rather than consuming this artifact |
| CoVerIf | Firmware-path partitioning, property slicing and incremental SAT | Firmware-guided reduction is treated as prior art | Not the exact transcript-versus-predicate artifact tested here |
| HIVE | Actual firmware/RTL verification with automatically validated hints | Cross-layer validated guidance is treated as prior art | Not the exact finite-domain certificate representation |
| Guo, Dutta and Jin | Proof carrying across the hardware/software boundary | Broad cross-layer proof carrying is treated as prior art | Different proof system and trusted compilation boundary |

Primary sources:

- Proof-Carrying Hardware via IC3: <https://arxiv.org/abs/1410.4507>
- Certifaiger: <https://fmv.jku.at/certifaiger/>
- Progress in Certifying Hardware Model Checking: <https://fmv.jku.at/papers/YuBiereHeljanko-CAV21.pdf>
- Btor2-Cert: <https://link.springer.com/chapter/10.1007/978-3-031-57256-2_7>
- SV-COMP 2026 rules: <https://sv-comp.sosy-lab.org/2026/rules.php>
- HW-CBMC and VerifOx: <https://www.cprover.org/verifox/>
- CoVerIf: <https://arxiv.org/abs/2001.01324>
- HIVE: <https://arxiv.org/abs/2309.08002>
- Proof carrying across the hardware/software boundary:
  <https://www.jinyier.me/papers/TIFS16_PCH.pdf>

No broad novelty claim survives this matrix. A positive result could establish
only a bounded representation and deployment advantage over the matched
baseline below.

## Frozen cohort

The cohort is the retained O2 OpenTitan PWM guarded-MMIO firmware:

- the complete eight-bit `a0` domain, inputs 0 through 255;
- six valid channel behaviors and one invalid behavior;
- the same firmware image and symbol layout used by the retained MMIO-to-RTL
  trust-boundary evidence; and
- the same fail-closed bounded RV32IMC semantics.

The expected firmware answer, source digests and exact work counts are learned
only through a clean run. They may be recorded after the first result, but no
threshold or route may then change.

## Matched routes

### GCC predicate certificate

The existing canonical predicate certificate carries six valid singleton
behaviors and one shared invalid-domain control trace. Its independent checker
fetches each lane's instruction bytes from the caller-owned image, enforces the
certified control path, replays all lane state updates and recovers every
result.

### Explicit execution transcript

The new baseline carries one ordered transcript for each of 256 inputs. Each
step records the program counter, register read and write masks, and ordered
memory reads and writes observed by the bounded machine. Terminal return,
ordered MMIO events, event program locations and step count are retained per
input.

The consumer:

1. parses canonical bytes under fixed size and count limits;
2. binds the exact caller-owned firmware image and symbol layout;
3. creates a fresh replay machine for each declared input;
4. executes and compares every step observation in order;
5. refuses early termination, omitted or extra steps, input duplication,
   noncanonical ordering and terminal-result drift; and
6. re-encodes accepted evidence to byte-identical bytes.

This baseline is intentionally redundant but independently checkable. It is
not a log parser and does not trust the transcript's terminal answer.

## Measurements

Both routes report:

- artifact bytes;
- producer and verifier decoded instruction transitions;
- scalar-equivalent lane operations where applicable;
- fresh-process consumer wall time and peak RSS;
- complete recovered behavior identity; and
- hostile mutation and truncation refusal counts.

One untimed warm-up precedes five fresh-process consumer trials. Artifact
production is measured separately and remains visible.

## Predeclared gates

The candidate survives this closest-system gate only if:

1. both routes recover byte-identical complete firmware semantics;
2. two clean cycles produce byte-identical artifacts and semantic summaries;
3. both consumers reject every retained single-field hostile case, every
   one-byte artifact mutation and every artifact truncation;
4. GCC verifier decoded transitions are strictly fewer than the explicit
   transcript verifier transitions;
5. GCC artifact bytes are at most half the explicit transcript artifact bytes;
6. GCC median warm-consumer wall time is no greater than the explicit
   transcript median;
7. GCC maximum peak RSS is no greater than twice the explicit transcript
   minimum; and
8. all producer costs, lane operations, payload sizes and adverse results
   remain visible.

The payload threshold is the primary representation test. The wall and memory
thresholds prevent a smaller artifact from hiding a material consumer penalty.
Failure of any gate is retained as a negative result.

## Claim boundary

Passing would show only that GCC's predicate certificate is a smaller,
lower-transition independently checkable representation than this faithful
explicit execution transcript for one frozen eight-bit compiled-firmware
cohort, without a measured warm-consumer regression.

It would not establish algorithmic novelty, universal compression, solver
superiority, production readiness, or an advantage over all proof-carrying
software and hardware systems. External assessment and broader cohorts would
remain necessary.

## Initial local result

The first Darwin arm64 O2 cycle passes the semantic, representation,
transition, warm-consumer wall-time and memory gates:

| Measure | GCC predicate | Explicit transcript | GCC advantage |
| --- | ---: | ---: | ---: |
| Artifact bytes | 7,339 | 2,440,530 | 332.54 times smaller |
| Producer decoded transitions | 4,408 | 115,960 | 26.31 times fewer |
| Verifier decoded transitions | 4,408 | 115,960 | 26.31 times fewer |
| Median warm-consumer wall time | 0.01 s | 0.02 s | 2.00 times lower |
| Maximum / minimum measured RSS | 6,225,920 bytes maximum | 22,282,240 bytes minimum | 3.58 times lower |

Both routes recover all 256 input behaviors with normalized semantic SHA-256
`b7bd4c74c62427a9c2c8ffe294960dfc8db61325299ebe15726b9d6934f15f9c`.
Two clean production cycles are byte-identical:

- GCC predicate artifact SHA-256:
  `b3fa53d46b09125383cb4e47d211444c461c9b8f526c319a87b01ec9050d146b`;
- explicit transcript SHA-256:
  `9ffccef0879de96c6e3c34aaf9635dd5728ebddd1fda04d666e4b11af90dc066`.

The GCC result still performs 112,000 scalar-equivalent lane operations. The
transition result therefore concerns shared instruction decoding, not constant
total semantic work. The explicit artifact also retains substantially more
consumer memory because it parses 2.44 MB of step observations.

At this initial stage, the result was promising but not yet an admitted
completed gate. The new codec
unit suite refuses mutation, truncation and source substitution, and the
repository's complete all-target test and strict Clippy matrices pass. The
predeclared exhaustive hostile check over every byte and truncation of the
2,440,530-byte authentic transcript has not yet completed. Producer wall time,
a retained reproducible result bundle and hosted Linux reproduction also
remained open. No conclusion was admitted before those controls closed below.

## Exhaustive hostile result

The authentic 2,440,530-byte transcript passes the literal finite hostile
matrix with eight local workers:

- 2,440,530 single-bit byte mutations refused;
- 2,440,530 truncations refused;
- one-byte extension refused;
- firmware-image drift refused; and
- symbol-layout drift refused.

All 4,881,063 cases refuse. Mutations take 341,805 milliseconds and
truncations take 169,508 milliseconds. This cost is retained because the
explicit artifact's size materially increases exhaustive qualification work.

## Hosted Ubuntu result

[Hosted run 30448898533](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30448898533)
passes on Linux x86-64 at master revision
`867f91be648deb1b528e4de2d35db691864047b2`.

Two clean production cycles are byte-identical and reproduce the Darwin
semantic and artifact identities exactly. Five warm-consumer trials per route
produce:

| Measure | GCC predicate | Explicit transcript | GCC advantage |
| --- | ---: | ---: | ---: |
| Artifact bytes | 7,339 | 2,440,530 | 332.54 times smaller |
| Verifier decoded transitions | 4,408 | 115,960 | 26.31 times fewer |
| Median warm-consumer wall time | 0.02 s | 0.04 s | 2.00 times lower |
| Maximum / minimum measured RSS | 8,340 KiB maximum | 20,192 KiB minimum | 2.42 times lower |

The hosted run retains both canonical artifacts, the rebuilt firmware inputs,
all ten consumer reports and resource measurements as a GitHub Actions
artifact.

## Admitted bounded conclusion

All eight predeclared gates pass for the frozen O2 cohort. GCC's predicate
certificate is therefore a substantially smaller, lower-decoding
independently checkable representation than a faithful explicit execution
transcript at the same complete eight-bit compiled-firmware boundary. The
advantage reproduces on Darwin arm64 and hosted Linux x86-64 without a
warm-consumer time or memory regression.

This is a measured closest-system result, not a broad novelty claim.
Proof-carrying software and hardware, firmware-guided RTL reduction and
cross-layer validation remain prior art. The surviving distinction is the
specific finite-domain predicate representation and its measured deployment
properties for this cohort. Broader firmware shapes, nonuniform control flow
and independent external evaluation remain open.
