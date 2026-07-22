# Caliptra watchdog experiment v1

Status: local arm64 and hosted amd64 baselines validated. Independent operator
acceptance remains open.

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
- [`caliptra-wdt-composed-witness-amd64-v1.csv`](../results/caliptra-wdt-composed-witness-amd64-v1.csv)
- [`caliptra-wdt-composed-witness-amd64-v1.manifest.txt`](../results/caliptra-wdt-composed-witness-amd64-v1.manifest.txt)
- [`caliptra-wdt-hosted-amd64-v1.provenance.txt`](../results/caliptra-wdt-hosted-amd64-v1.provenance.txt)

[Hosted run 29863532960](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29863532960)
passes the complete baseline on amd64. Its nine-row CSV is byte-identical to
arm64, and its manifest differs only in the expected architecture-specific
rIC3 binary and Certifaiger tree hashes. The retained provenance binds both
files to the workflow commit and artifact digest.

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

## Predeclared word-level generalisation probe

The next probe exports the same pinned module and fixed cascade configuration
to canonical BTOR2 without a bounded frame counter. It asks the existing GCC
predicate-set portfolio to check the three native timeout properties at
horizons 2, 3, 5, and 1,000,000,000. No Caliptra-specific recogniser or route
hint is permitted. Existing specialised admission may succeed, ordinary exact
fallback may answer within its static limits, or the governed portfolio may
refuse. Every outcome is retained. The probe tests generalisation of the
word-level backend; it is not part of the completed AIGER baseline.

### Retained word-level result

The unchanged Yosys export first exposed missing support for the standard BTOR2
`concat` and `redand` operations. GCC now parses, type-checks, evaluates, and
tracks input support through both operations with explicit malformed-width and
unknown-operand tests. The canonical model then inspects successfully with 34
nodes, one semantic input, two states, and three bad properties.

The existing predicate-set portfolio refuses all four frozen queries. Horizons
2, 3, and 5 reach exact-search fallback, which correctly refuses because the
asynchronous-reset lowering makes the bad expressions depend on the current
input. The billion-frame query reaches the existing search-horizon refusal
first. Search certificate v1 records transition inputs but no distinct input
for evaluating the terminal bad frame. Broadening v1 would therefore change
witness semantics and is prohibited by the compatibility policy.

The result is retained in
[`caliptra-wdt-word-probe-v1.csv`](../results/caliptra-wdt-word-probe-v1.csv).
The next candidate is an additive search certificate v2 with an explicit
terminal-frame input and complete two-valued bad checks for every SAFE layer.
V1 decoding and verification must remain unchanged.

That candidate is now implemented as
[bounded search certificate v2](BTOR2_BOUNDED_SEARCH_V2.md). The frozen
horizons 2, 3, and 5 complete through ordinary exact search for all three
Caliptra properties. At horizon 2 all are SAFE. At horizon 3 the first-stage
timeout is UNSAFE at frame 3 while the second-stage and fatal properties are
SAFE. At horizon 5 all three are UNSAFE, at frames 3, 5, and 5 respectively.
The billion-frame query still refuses at the static search-horizon limit and
returns no answer.

The retained v2 probe regenerates the canonical model and certificates twice,
verifies every certificate independently from the source, checks those exact
answers, and keeps the billion-frame refusal as a negative control. This is an
interoperability result for explicit bounded search, not recurrence
acceleration or a Caliptra-specific solver route.
