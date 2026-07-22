# OpenTitan prim_count closest baseline v1

## Scope

This comparison uses the same pinned semantic-changing OpenTitan revisions,
selected counter configuration, reset environment, and safety predicate as the
GCC revision-local experiment. Pinned Yosys generates distinct AIGER models.
Qualified rIC3 produces the old SAFE witness and new UNSAFE trace. Qualified
Certifaiger, LRAT checking, and AIGER simulation independently validate them.

## Result

| Measurement | Value |
|---|---:|
| Before model | 252 bytes |
| After model | 252 bytes |
| Models identical | false |
| Old SAFE witness | 192 bytes |
| New UNSAFE trace | 13 bytes |
| Old witness valid for new model | false |
| New trace valid for old model | false |
| Evidence regenerated for revision | 13 bytes |

Two clean isolated runs produced byte-identical CSV and manifest files:

- [`opentitan-prim-count-closest-baseline-arm64-v1.csv`](../results/opentitan-prim-count-closest-baseline-arm64-v1.csv)
- [`opentitan-prim-count-closest-baseline-arm64-v1.manifest.txt`](../results/opentitan-prim-count-closest-baseline-arm64-v1.manifest.txt)

## Interpretation

The semantic revision invalidates the old whole-model witness, so the
identical-model shortcut that falsified the PLIC experiment no longer applies.
GCC demonstrates a different capability: it retains and reverifies the
unchanged environment relation while recomputing the changed component.

This does not produce a cost breakthrough on the tiny counter. The maintained
baseline regenerates a complete 13-byte counterexample, while GCC's complete
retained portfolio is about 1.7 MB. GCC provides explicit local attribution,
complete local relations, and reusable independently checked sections, but the
artifact-size premium is several orders of magnitude. The result therefore
qualifies the functional distinction and falsifies a certificate-byte advantage
for this cohort.

The next novelty gate must use a larger repeated-revision workload where local
relation reuse avoids materially more recomputation than a maintained
whole-model producer, including model construction and independent checking.
No novelty claim is justified by this result alone.
