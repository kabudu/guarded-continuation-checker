# Caliptra watchdog experiment v1

Status: local arm64 baseline validated. Hosted amd64 reproduction remains open.

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

The deterministic answer-only boundary probe found the first timer-one bad
frame at 3 and the first timer-two and fatal bad frames at 5. Horizons 2, 3,
and 5 are therefore frozen before comparative timing or evidence-size
measurement. This gives five SAFE and four UNSAFE individual rows. Horizon 2
has a three-property shared SAFE set, while horizon 3 has a two-property shared
SAFE set containing timer two and fatal timeout.

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

## Local retained result

All nine individual answers match the frozen boundary matrix. The five SAFE
results are accepted by Certifaiger with every generated SAT proof checked by
`lrat_isa`. The four UNSAFE traces replay with `aigsim` and terminate at the
first bad frame. Two clean source-to-AIGER exports and two clean evidence runs
are byte-identical. Six hostile controls reject malformed and truncated SAFE
evidence, wrong-model SAFE evidence, a wrong shared model, a truncated UNSAFE
trace, and an UNSAFE trace substituted into a SAFE horizon.

At horizon 2, three individual SAFE witnesses total 43,630 bytes. Their verified
composition is 13,570 bytes, a 68.90% reduction. At horizon 3, the timer-two and
fatal witnesses total 29,126 bytes. Their verified composition is 13,432 bytes,
a 53.89% reduction. This is evidence that established FM 2026 composition works
on a second public embedded design. It is not an algorithmic novelty result.

Retained data:

- [`caliptra-wdt-composed-witness-v1.csv`](../results/caliptra-wdt-composed-witness-v1.csv)
- [`caliptra-wdt-composed-witness-v1.manifest.txt`](../results/caliptra-wdt-composed-witness-v1.manifest.txt)

Reproduce locally with the qualified rIC3 and Certifaiger tool trees:

```sh
scripts/benchmark-caliptra-wdt-composed-witness-v1.sh \
  target/release/guarded-continuation-checker \
  "$(command -v yosys)" \
  /tmp/ric3-output \
  /tmp/certifaiger-output \
  /tmp/caliptra-wdt.csv \
  /tmp/caliptra-wdt.manifest.txt
```
