# Proof-carrying controller MTBDD maintained-tool baseline v1

## Question

Does the proof profile outperform a maintained formal workflow when both check
the identical controller, physical plant, six properties, initial states,
wiring, and bounded horizons?

## Frozen scope

`scripts/benchmark-controller-proof-mtbdd-maintained-baseline.sh` compares proof
artifact creation and self-check, verification by a fresh GCC process, and one
SymbiYosys bounded-model-checking session using maintained Yosys and Z3.

Every accepted row must reproduce four UNSAFE properties at shortest frames 4,
7, 15, and 15, plus two SAFE properties through frame 32. The harness rejects
changed answers, missing members, invalid trial counts, unsupported resource
measurement platforms, or an existing output file. It records whole-process
wall time, peak RSS, portable evidence bytes, platform, and timing backend.

The formal path reruns the model and emits no portable replay artifact. The GCC
path consumes a 251,221-byte source-bound artifact containing an independently
checked controller equivalence proof and all six bound plant results. This is an
identical-query comparison, not an identical-evidence comparison.

## Retained result

Five release-build trials on macOS arm64 used Rust 1.97.0, Yosys 0.67+post at
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`, Z3 4.16.0, and SymbiYosys at
`fea6e467d067b3ea84b6b5ac08cd48beb59f0d42`.

| Process | Median wall time | Median peak RSS | Portable evidence |
| --- | ---: | ---: | ---: |
| Proof creation and self-check | 1.46 s | 22,167,552 bytes | 251,221 bytes |
| Fresh proof verification | 0.48 s | 10,092,544 bytes | 251,221 bytes |
| Maintained formal oracle | 0.36 s | 29,294,592 bytes | none |

All fifteen rows agree exactly. The maintained oracle is 1.33 times faster than
fresh proof verification and about 5.39 times faster than initial proof creation
plus verification. Proof verification uses 65.5% less peak RSS and delivers a
portable artifact that can be checked without Yosys, Z3, or producer state.

This is a negative runtime result and a positive evidence-transfer and consumer
memory result. It does not establish novelty. The raw observations are retained
in `results/controller-proof-mtbdd-maintained-baseline-v1.csv`.

GitHub-hosted Linux x86_64 run
[29734089858](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29734089858)
reproduces the direction. Proof creation takes 4.22 s, fresh proof verification
takes 0.88 s, and the maintained oracle takes 0.53 s. The oracle is 1.66 times
faster than verification and about 9.62 times faster than creation plus
verification. Proof-verifier peak RSS is 6,828,032 bytes versus 33,746,944 bytes
for the oracle, a 79.8% reduction. The three exact-answer rows are retained in
`results/controller-proof-mtbdd-maintained-baseline-linux-v1.csv`.

## Remaining boundary

Peak RSS and timing vary by host, but both measured platforms reproduce the
runtime loss and verifier-memory advantage. The formal oracle does not emit
evidence with the same trust-transfer semantics, so this experiment cannot claim
that GCC beats an equivalent external certificate. An established certifying
hardware flow, independent expert review, and non-repository-authored designs
remain open.
