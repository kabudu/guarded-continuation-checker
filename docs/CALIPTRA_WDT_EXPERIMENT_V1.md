# Caliptra watchdog experiment v1

Status: predeclared, implementation in progress. No results have been retained.

## Purpose

This experiment tests GCC against a second materially distinct public embedded
RTL design. It uses the CHIPS Alliance Caliptra two-stage watchdog timer, not
another OpenTitan component or repository-owned controller.

## Frozen design and configuration

The unmodified Apache-2.0 source, exact upstream revision, and digest are bound
in [`corpus/rtl/caliptra-wdt/PROVENANCE.md`](../corpus/rtl/caliptra-wdt/PROVENANCE.md).
The GCC-owned wrapper fixes the module in cascade mode with timer-one timeout 3
and timer-two timeout 2. Reset remains a semantic input. The bounded frame
instrumentation selects three properties independently:

1. timer one has not timed out;
2. timer two has not timed out;
3. no fatal cascade timeout has occurred.

The tested horizons will be chosen only by a deterministic boundary probe and
then frozen before comparative timing or evidence-size measurements. The final
set must contain both SAFE and UNSAFE outcomes and at least one shared SAFE set
eligible for witness composition.

## Acceptance criteria

The cycle passes only if:

- GCC and the pinned maintained AIGER route agree on every answer;
- every SAFE result is independently checked by Certifaiger plus `lrat_isa`;
- every UNSAFE result is replayed with `aigsim` at its first bad frame;
- two clean exports and two clean evidence runs are byte-identical;
- malformed, truncated, wrong-bound, and wrong-model evidence is rejected;
- the complete baseline passes locally and in hosted amd64 Linux;
- source, model, toolchain, workflow, and retained-result provenance is bound.

Failure or loss is retained and reported. This experiment is an evidence-breadth
gate. It cannot by itself supply independent operator acceptance or establish a
novel verification algorithm.
