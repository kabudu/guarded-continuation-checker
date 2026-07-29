# OpenSBI UART 8250 dense decode qualification v1

## Question

Does GCC's dense decode graph preserve exact firmware behaviour on a second
unrelated public project whose execution includes byte, halfword and word
MMIO helpers, baud-divisor arithmetic, status polling and serial output?

This experiment uses OpenSBI's UART 8250 implementation. It does not modify
the graph format, dense verifier, RV32IMC execution policy, scalar replay
semantics or refusal boundary.

## Frozen source and boundary

- upstream repository: `https://github.com/riscv-software-src/opensbi`;
- upstream release: `v1.8.1`;
- upstream source: `lib/utils/serial/uart8250.c`;
- upstream licence: BSD-2-Clause;
- real functions under test: `uart8250_device_init`,
  `uart8250_device_putc` and `uart8250_device_getc`; and
- scalar input space: all 256 `a0` values.

The qualification retrieves the exact pinned source and licence, verifies
their frozen SHA-256 identities and compiles the source without editing it.
GCC-authored compatibility headers and the bounded caller remain outside the
upstream directory.

The caller maps input bits to the supported register widths 1, 2 and 4, three
register shifts, zero or nonzero baud configuration, UART capability flags,
receive-ready state and the transmitted byte. The bounded in-memory UART
register bank always begins transmitter-ready, so the real polling loop must
terminate. Final register state and receive result become canonical bounded
events.

## Frozen comparison

For one pinned `-O2` RV32IMC image, compare:

1. the dense-index graph verifier;
2. the retained balanced-tree graph verifier; and
3. the canonical trace-family verifier.

Run five warm whole-process trials per route. Preserve exact graph bytes,
terminal reconstruction, scalar work, unique decode work, edge coverage and
all losing measurements.

## Predeclared gates

The qualification succeeds only if:

1. two clean builds produce byte-identical firmware, symbol and graph files;
2. all three routes return identical canonical terminal semantics for all 256
   inputs;
3. dense and tree verification return identical scalar work, unique decode
   counts and graph edge counts;
4. every declared graph edge is covered by independent scalar replay;
5. the retained graph hostile qualifier refuses every generated case;
6. dense median elapsed time is no greater than the tree median;
7. dense median elapsed time is no more than 1.25 times the trace-family
   median;
8. dense median peak RSS is no more than 1.25 times trace-family median peak
   RSS; and
9. source, compiler, verifier, trial count, statistic and thresholds remain
   unchanged after the first complete resource cycle.

If any register width, arithmetic path or polling operation lies outside the
existing bounded executor, refusal is the result. The interpreter must not be
weakened to force admission.

## Claim boundary

Passing would establish transfer to one pinned OpenSBI UART source under a
GCC-authored bounded caller. It would add evidence for width-polymorphic MMIO
and bounded polling, not arbitrary OpenSBI support, universal performance,
production qualification or release readiness.

## Local result

The qualification is refused before graph construction, so it does not reach
the representation or resource gates.

The initial pre-measurement caller combined halfword register access with a
byte register stride for input 1. That is not a hardware-valid 8250 layout and
caused the real transmitter-ready loop to exceed the bounded step policy. The
caller was corrected to retain all three widths and shifts while ensuring
width-aligned register addresses.

With that correction, inputs 0 through 15 are admitted. Input 16 selects the
frozen nonzero-baud path. OpenSBI's exact baud-divisor expression compiles to
RV32M `DIVU` at program counter `0x800000bc`. The existing bounded executor
refuses that instruction:

```text
compiled-MMIO decode graph: input 16: bounded RV32IMC extraction:
unsupported instruction at 0x800000bc: Divu(RType(47568307))
```

No timing cycle was run and no graph, trace-family or explicit artifact was
claimed. Removing the nonzero-baud path after observing the refusal would
change the frozen cohort. Adding division support inside this qualification
would change the predeclared executor boundary. Both are disallowed.

The result isolates a concrete product gap: exact RV32M division semantics are
required before GCC can qualify this public OpenSBI UART path. Any such support
must be specified, implemented and tested as a separate experiment before this
unchanged cohort is retried.
