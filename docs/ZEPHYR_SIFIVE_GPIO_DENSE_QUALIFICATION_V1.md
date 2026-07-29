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

