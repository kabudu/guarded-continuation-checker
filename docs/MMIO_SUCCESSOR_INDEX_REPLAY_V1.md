# MMIO successor-index replay v1

Status: predeclared, not yet measured.

## Question

Can an artifact-compatible verifier resolve every declared program-counter edge
to a node index once, then carry that index through scalar replay, while
preserving the exact checks and reducing the dense graph's consumer cost?

## Frozen mechanism

The new route uses the existing version-2 decode-graph bytes unchanged. During
preflight it:

1. validates and decodes every node exactly as the current dense route does;
2. builds the same bounded halfword index;
3. converts each declared successor program counter to either a checked node
   index or a terminal sentinel; and
4. rejects a terminal sentinel unless the replay machine is complete after
   taking that edge.

For each input, replay resolves the initial program counter through the dense
index once. Each later step selects a declared successor by program counter
and carries its prevalidated node index directly into the next iteration.

The route must retain source hashing, instruction-byte comparison, canonical
ordering, complete edge coverage, terminal reconstruction, scalar-work
accounting and every existing resource bound. It must not trust a producer
supplied index or change the wire format.

## Frozen cohort and baselines

The cohort is the pinned OpenSBI v1.8.1 UART 8250 qualification with its
hardware-aligned 256-input caller and the separately merged exact RV32M
division semantics.

The comparison routes are:

- `graph`, the existing dense halfword lookup on every scalar step;
- `graph-successor`, the mechanism above; and
- `trace-family`, the retained canonical trace-family verifier.

Each resource invocation performs exactly 100 complete verifications in one
process after one unmeasured warm-up. There are five trials per route. The
median is selected independently for elapsed time and peak RSS. Route order is
rotated across trials. The implementation, cohort, repetition count, trial
count, statistic and thresholds freeze before the first complete resource
cycle.

## Predeclared gates

The experiment succeeds only if:

1. all three routes preserve identical terminal semantics and scalar work for
   all 256 inputs;
2. both graph routes report identical node, edge and edge-coverage counts;
3. every retained hostile graph case is refused by `graph-successor`;
4. successor replay median elapsed time is no more than 0.80 times the current
   dense median;
5. successor replay median elapsed time is no more than 1.25 times the
   trace-family median;
6. successor replay median peak RSS is no more than 1.10 times the current
   dense median; and
7. two clean builds reproduce identical firmware, symbols, graph and
   trace-family artifacts.

## Claim boundary

Passing would establish a consumer-side replay improvement for the pinned
OpenSBI cohort without changing evidence bytes. It would not establish
arbitrary firmware support, universal performance, production qualification
or release readiness. Failure is retained without changing the thresholds or
rerunning a tuned cohort.
