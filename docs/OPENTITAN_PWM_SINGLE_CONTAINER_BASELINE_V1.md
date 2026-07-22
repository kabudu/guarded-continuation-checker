# OpenTitan PWM single-container maintained baseline v1

Status: qualified locally on arm64, 2026-07-22

## Question

The v0.30.0 maintained-tool baseline starts a separate Docker container for
each of 20 producer jobs and each of 20 checker jobs. This experiment asks how
much of GCC's measured source-through-answer advantage remains when container
startup is removed from the per-job path.

It does not call the maintained tools a warm service. Each rIC3, Certifaiger,
and `aigsim` invocation remains a fresh process. The controlled change is that
all producer processes run in one network-disabled container and all checker
processes run in one network-disabled container.

## Frozen cohort

The experiment uses the same OpenTitan-derived PWM cohort as
`OPENTITAN_PWM_CROSSTALK_REVISION_IMPACT_V1`:

- four source combinations;
- five bounded properties per combination;
- 20 observations in total;
- 9 expected SAFE answers and 11 expected UNSAFE answers;
- three expected minimal semantic change sets;
- model-set SHA-256
  `1e9c81c03f78b32b266c5d367cf484c1e56deba0808d1c4c59d460cb47d65e0e`.

The pinned Yosys, rIC3, Certifaiger, `aigsim`, container images, resource
measurement helper, source files, and property horizons remain unchanged from
the qualified isolated-container baseline.

## Modes

Two maintained-tool modes are measured:

1. `single-container-sequential`: one producer container executes the 20 rIC3
   processes sequentially, followed by one checker container executing the 20
   independent checks sequentially.
2. `single-container-parallel-4`: one producer container executes at most four
   rIC3 processes concurrently, followed by one checker container executing at
   most four independent checks concurrently.

Parallel scheduling uses a fixed lexical job order and a fixed concurrency of
four. Every child writes a separate log and evidence file. The harness waits
for all children and fails if any child fails.

## Timing and resource scope

For each mode, the harness records:

- native Yosys synthesis time and peak RSS;
- producer-container wall time, including its single container startup;
- source-through-producer wall time as synthesis plus producer orchestration;
- checker-container wall time, including its single container startup;
- maximum individual producer and checker child-process RSS, reported by the
  same `child-rusage-v1` helper used by the isolated-container baseline;
- container count and configured concurrency.

The parallel result is a throughput comparison, not a lower-compute claim.
Individual child RSS is not aggregate parallel RSS, so no total parallel-memory
comparison is claimed from that field.

Five clean trials are run for each mode. Medians are computed only after all
ten trials pass. No trial may be discarded or repeated selectively. A failed
trial invalidates the experiment until its cause is documented and the entire
ten-trial matrix is rerun.

## Qualification gates

Both modes must satisfy every gate in every trial:

- produce exactly 20 answers, 9 SAFE and 11 UNSAFE;
- match GCC's complete answer matrix;
- emit exactly 20 evidence artifacts;
- independently validate every SAFE witness and UNSAFE trace;
- reproduce the frozen model-set hash;
- reproduce one identical evidence-set hash across both modes and all trials;
- report non-zero timing and child RSS measurements;
- use exactly one producer container and one checker container per trial;
- run with networking disabled and read-only tool and model mounts;
- leave the original isolated-container results unchanged.

The performance result has no pass threshold. A slower maintained-tool result
is evidence for GCC's orchestration advantage. An equal or faster result
weakens or removes the v0.30.0 performance claim and must be reported with the
same prominence. Artifact size remains the already disclosed negative result
for GCC unless the encoded artifacts themselves change.

## Reporting rule

The retained result must report medians for all three maintained modes:

- isolated containers from the frozen v1 baseline;
- single-container sequential;
- single-container parallel with concurrency four.

It must also compare both new modes with the matched GCC result while clearly
separating synthesis, producer orchestration, independent checking, container
startup, and proof-artifact size. This experiment can strengthen or weaken a
product-value claim, but it cannot by itself establish algorithmic novelty.

## Qualified result

All ten predeclared trials passed without a selective rerun. Every trial
reproduced the frozen 20-answer matrix, the model-set hash, and evidence-set
SHA-256
`d38d815128058d44282c2a34b6c9a1e84cf02cb9a337e5e8a7206576a97da90f`.

The five-trial medians are:

| Mode | Source through producer | Producer orchestration | Independent checking |
| --- | ---: | ---: | ---: |
| Isolated containers, frozen v1 | 4.84 s | 4.229 s | 4.976 s |
| Single-container sequential | 0.89 s | 0.339 s | 0.937 s |
| Single-container parallel-4 | 0.82 s | 0.249 s | 0.750 s |
| GCC matched aggregate | 0.09 s | 0.01 s | below host timer precision |

Against the controlled maintained-tool modes, GCC's source-through-producer
result is about 9.89 times faster than sequential orchestration and 9.11 times
faster than four-way parallel orchestration. The v0.30.0 comparison against
isolated containers was 53.78 times. The advantage survives, but this result
shows that most of the earlier ratio was container-launch overhead rather than
GCC computation.

The maintained package remains 15,479 bytes and the GCC aggregate remains
128,768 bytes, so GCC still transfers about 8.32 times more data. Median
source-through-producer peak RSS for the two new modes is about 22.6 MB because
native synthesis dominates. This is higher than GCC's retained 15.9 MB median.
Parallel aggregate memory was not measured, so the individual child RSS fields
must not be used as a total-memory claim.

Retained evidence:

- [`opentitan-pwm-single-container-baseline-arm64-v1.csv`](../results/opentitan-pwm-single-container-baseline-arm64-v1.csv)
- [`opentitan-pwm-single-container-baseline-arm64-v1.summary.csv`](../results/opentitan-pwm-single-container-baseline-arm64-v1.summary.csv)
- [`opentitan-pwm-single-container-baseline-arm64-v1.manifest.txt`](../results/opentitan-pwm-single-container-baseline-arm64-v1.manifest.txt)

Reproduce the complete matrix with already qualified local tools:

```console
mkdir /tmp/gcc-pwm-single-container-matrix
scripts/run-opentitan-pwm-single-container-matrix-v1.sh \
  /path/to/pinned/yosys /path/to/ric3-output \
  /path/to/certifaiger-output \
  /tmp/trials.csv /tmp/summary.csv /tmp/manifest.txt \
  /tmp/gcc-pwm-single-container-matrix
```
