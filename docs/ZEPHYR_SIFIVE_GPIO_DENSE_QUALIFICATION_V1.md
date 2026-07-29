# Zephyr SiFive GPIO dense decode qualification v1

## Question

Does GCC's dense source index preserve its controlled OpenTitan PWM result on
an unrelated public firmware source with different authors, framework,
peripheral and control-flow structure?

This qualification uses the Zephyr SiFive GPIO driver. It does not modify the
dense verifier, graph format, scalar replay semantics or refusal policy.

## Frozen source and boundary

- upstream repository: `https://github.com/zephyrproject-rtos/zephyr`;
- upstream release: `v4.2.0`;
- upstream source: `drivers/gpio/gpio_sifive.c`;
- licence: Apache-2.0;
- public functions under test: pin configuration plus raw masked, set, clear
  and toggle operations; and
- scalar input space: all 256 `a0` values.

The upstream source and licence are retained verbatim. GCC-authored
compatibility headers and a freestanding caller remain outside the upstream
directory. The caller exposes the real static Zephyr driver functions within
their original translation unit and records the resulting register state as
bounded events.

## Frozen comparison

For one pinned `-O2` RV32IMC image, compare:

1. the dense-index graph verifier;
2. the retained balanced-tree graph verifier; and
3. the canonical trace-family verifier.

Run five warm whole-process trials per route. Preserve exact graph bytes,
terminal reconstruction, scalar path work, unique decode work, edge coverage
and all losing measurements.

## Predeclared gates

The qualification succeeds only if:

1. a clean rebuild produces byte-identical firmware, symbol and graph files;
2. all three routes return identical canonical terminal semantics for all 256
   inputs;
3. dense and tree verification return identical scalar work, unique decode
   counts and graph edge counts;
4. every dense graph edge is covered by independent scalar replay;
5. the retained graph hostile corpus refuses every mutation under the dense
   verifier;
6. dense median elapsed time is no greater than the tree median;
7. dense median elapsed time is no more than 1.25 times the trace-family
   median;
8. dense median peak RSS is no more than 1.25 times trace-family median peak
   RSS; and
9. no threshold, trial count, statistic, source, compiler or verifier changes
   after the first complete resource cycle.

If the pinned public source cannot be represented within the existing bounded
RV32IMC execution policy, that refusal is the result. The experiment must not
weaken the interpreter to force admission.

## Claim boundary

Passing would show that the dense verifier result transfers to one unrelated,
pinned public driver under a GCC-authored bounded caller. It would not
establish arbitrary Zephyr support, universal speed, production qualification
or release readiness.

## Local result

The qualification passes every predeclared local gate.

Two clean builds produced identical firmware, symbol and graph bytes. The
pinned source and licence matched their frozen SHA-256 identities. The
resulting 256-input image has 16 canonical terminal behaviours:

- dense graph: 11,450 bytes, 94 nodes, 99 edges and 94 unique instruction
  decodes;
- trace family: 27,620 bytes, 16 traces and 1,324 decoded transitions; and
- explicit transcript: 458,148 bytes and 21,184 decoded transitions.

All routes independently replay the same 21,184 scalar path steps. Dense and
tree-backed graph verification agree on all 94 nodes, 99 edges, scalar work
and canonical terminals. The dense verifier covers every declared edge.

The retained hostile qualifier refused 22,908 cases: every single-bit
artifact mutation, every truncation, an extension, image drift, symbol drift,
missing/additional/duplicate edges, a missing node and terminal drift.

The first and only frozen five-trial resource cycle produced these medians on
Darwin arm64:

| Route | Median elapsed | Median peak RSS |
| --- | ---: | ---: |
| Dense graph | 0.01 s | 4,079,616 bytes |
| Tree graph | 0.01 s | 4,112,384 bytes |
| Trace family | 0.01 s | 4,259,840 bytes |

The elapsed results are at local timer resolution, so they establish
non-regression rather than a speedup. Dense median memory is below both
baselines.

The dedicated hosted Linux x86-64 qualification in
[run 30481063544](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30481063544)
also passes. It rebuilt the source and exact artifacts twice, refused the
hostile corpus and enforced the frozen resource gates:

| Route | Hosted median elapsed | Hosted median peak RSS |
| --- | ---: | ---: |
| Dense graph | 0.27 s | 10,596,352 bytes |
| Tree graph | 0.27 s | 10,592,256 bytes |
| Trace family | 0.27 s | 10,510,336 bytes |

Hosted durations also establish non-regression rather than a speedup. Dense
median RSS is 1.008 times the trace-family median and remains within the
predeclared 1.25 limit. The complete protected matrix, including the focused
qualification job, passed before merge.

The public-source transfer result is therefore reproduced on both local macOS
arm64 and hosted Linux x86-64. The production support profile remains
unchanged.
