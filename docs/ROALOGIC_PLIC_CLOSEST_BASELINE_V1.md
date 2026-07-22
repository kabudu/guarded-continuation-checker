# Roa Logic PLIC closest-system baseline v1

## Result

The revision-local novelty hypothesis is falsified for the first public PLIC
revision pair.

The two pinned source revisions have different source SHA-256 values, but
pinned Yosys produces byte-identical whole-circuit AIGER models for both the
SAFE and UNSAFE properties. A SAFE witness and an UNSAFE frame-two trace
produced once against the older revision are independently accepted against
both revisions. The established model-level route therefore regenerates zero
semantic model or evidence bytes after this source change.

This is stronger disconfirming evidence than comparing GCC only with a full
rebuild. GCC reduces its changed-side candidate work from 4,100 valuations to
four when the monitor changes, and can retain the monitor relation when the
PLIC source changes. It cannot claim a revision-local evidence advantage for
this PLIC pair because the closest baseline retains all semantic evidence.

## Identical scope

The baseline uses the same two authentic PLIC source revisions, the same
unconstrained five environmental inputs, the same pending monitor, and the
same two answer classes:

- the impossible property is SAFE;
- repeated pending is UNSAFE first at frame two.

The AIGER SAFE proof is unbounded, so it is not weaker than GCC's bounded SAFE
answer. The UNSAFE trace preserves the same earliest frame. The wrapper and
properties remain clearly labelled GCC-authored evaluation material.

Source-to-model attestation still changes because the source digest changes.
That metadata cost applies independently of semantic proof reuse and is not
reported as regenerated semantic evidence.

## Independent qualification

The deterministic builder pins Yosys commit
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`, reconstructs both exact upstream
source byte sequences, builds each property model, and requires old and new
models to compare byte-for-byte.

Pinned rIC3 produces one 84-byte SAFE witness and one 37-byte UNSAFE trace.
Qualified Certifaiger 10.2.0, with its `lrat_isa` proof path, accepts the SAFE
witness against both 177-byte models. The qualified AIGER simulator accepts the
UNSAFE trace against both 1,590-byte models. Tool and artifact digests are in
[`roalogic-plic-closest-baseline-arm64-v1.manifest.txt`](../results/roalogic-plic-closest-baseline-arm64-v1.manifest.txt).

Reproduce with:

```sh
scripts/benchmark-roalogic-plic-closest-baseline-v1.sh \
  target/release/guarded-continuation-checker \
  /path/to/pinned/yosys \
  /path/to/qualified/ric3-output \
  /path/to/qualified/certifaiger-output \
  /tmp/roalogic-plic-closest-baseline-v1.csv \
  /tmp/roalogic-plic-closest-baseline-v1.manifest.txt
```

## Consequence

The current public pair does not satisfy the predeclared semantic-revision
control. It changes literal widths in source without changing the synthesized
transition system. It remains useful for source-binding, hostile-input, API,
and operational testing, but it cannot support the candidate novelty claim.

The next comparison must use at least two authentic revisions that change
reachable transition semantics while preserving a stable component interface.
The same maintained model, SAFE-witness, and UNSAFE-trace baseline must then be
rerun. No novelty claim can advance until that harder comparison passes.
