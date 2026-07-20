# Reproducibility

## Proof-carrying MTBDD self-service profile

```sh
cargo build --release --locked
scripts/run-controller-proof-mtbdd-self-service-acceptance.sh \
  target/release/guarded-continuation-checker \
  target/controller-proof-acceptance.csv
scripts/benchmark-controller-proof-mtbdd-process.sh \
  target/release/guarded-continuation-checker \
  target/controller-proof-process.csv
scripts/benchmark-controller-proof-mtbdd-resources.sh \
  target/release/guarded-continuation-checker \
  target/controller-proof-resources.csv
```

Both scripts refuse to overwrite output. Set `TRIALS` from 1 to 20 for the
process benchmark; the retained result uses the default five.

## Environment

- Rust 1.97.0 (pinned by `rust-toolchain.toml`)
- Cargo
- Release mode for timings

```sh
cargo test
cargo build --release
```

Timing results vary by CPU and operating system. Correctness criteria are exact
agreement and witness validity; speed claims should be rerun locally.

## Certified causal counterexamples

Generate and separately replay-verify an atomic evidence bundle:

```sh
scripts/run-causal-analysis.sh \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  target/causal/infusion-pump 8 16
```

Verify the two checked-in research results without regenerating their timings:

```sh
target/release/guarded-continuation-checker verify-aiger-causal-bundle \
  examples/aiger/spi-bus-receive-e-08-bits.aag \
  results/causal-analysis-v1/spi

target/release/guarded-continuation-checker verify-aiger-causal-bundle \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  results/causal-analysis-v1/infusion-pump
```

The verifier re-solves sufficiency and every 1-minimality obligation. Timing
fields are observations from the originating host and are not expected to match.
See [causal evidence bundle v1](CAUSAL_BUNDLE_V1.md) for the exact contract.

Compare deletion and QuickXplain on the identical intervention workload:

```sh
target/release/guarded-continuation-checker \
  benchmark-aiger-causal-strategies \
  examples/aiger/causal-sparse-16.aag \
  1 16 target/causal-sparse-strategies.csv
```

Repeat with `causal-dense-16.aag`, the infusion-pump regression at horizon 8,
and the SPI fixture at horizon 16 to reproduce the cohort described in
[the closest-method comparison](CAUSAL_STRATEGY_COMPARISON.md).

## Modular DIMACS result

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-dimacs \
  examples/modular-demo.cnf 10000 10 results/reproduced-modular.csv
```

## SATLIB gate coverage

Obtain SATLIB instances from an authorized source and run the DIMACS command for
each local file. Third-party formulas are not redistributed in this repository.
The curated aggregate result is `results/continuation-dimacs-summary-v1.csv`.

## Scaling

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-quotients \
  banded-planted 100 4 98302 1 results/reproduced-scaling.csv
```

## Repeated queries

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-reuse \
  banded-planted 100 4 98302 20000 results/reproduced-reuse.csv
```

## Wide assumptions

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-reuse-stress \
  banded-planted 100 4 98302 20000 40 results/reproduced-stress.csv
```

## Update repairs

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-repairs \
  banded-planted 100 4 98302 20 200 results/reproduced-repairs.csv
```

Deletion rows intentionally rebuild from layer zero. Do not interpret the older
v1 deletion summary from the historical experiment repository as a valid generic
speed result; `continuation-repairs-summary-v2.csv` is authoritative.

## Temporal bounded-width phase

```sh
./target/release/guarded-continuation-checker \
  benchmark-continuation-temporal-phase \
  2,4,6 10,100,1000,10000 100 12 424242 \
  results/reproduced-temporal-long.csv

./target/release/guarded-continuation-checker \
  benchmark-continuation-temporal-phase \
  8 10,100,1000 100 12 424242 \
  results/reproduced-temporal-width8.csv

./target/release/guarded-continuation-checker \
  benchmark-continuation-temporal-phase \
  10 10,100 100 12 424242 \
  results/reproduced-temporal-width10.csv
```

The 12-bit limit is fixed for the complete sweep. Widths 12--20 are recorded as
structurally rejected rather than trial-solved. The dense quotient and exact
repeated-transition kernel use identical queries and are independently checked
against persistent Varisat.

## CNF-recognized transition vocabulary

```sh
./target/release/guarded-continuation-checker \
  benchmark-temporal-vocabulary \
  copy,negate,permute,xor,circuit \
  4,6,8 10,100,1000 100 8 777 \
  results/reproduced-temporal-vocabulary-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-temporal-vocabulary \
  copy,negate,permute,xor,circuit \
  5,7 37,333,2000 100 8 99991 \
  results/reproduced-temporal-vocabulary-holdout.csv
```

The maximum width of eight is fixed before the runs. Recognition time is included
in the reported break-even calculation. `agreement` and `witnesses_valid` must be
true for every admitted row.

## Exact composed transitions

```sh
./target/release/guarded-continuation-checker \
  benchmark-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 10,100,1000 100 8 12345 \
  results/reproduced-temporal-compositions-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  5,7 37,333,2000 100 8 987654 \
  results/reproduced-temporal-compositions-holdout.csv
```

The width-eight gate is fixed before both runs. Recognition time includes
template verification, exhaustive determinism checking, and jump-table
construction. All admitted rows must report `agreement=true`,
`witnesses_valid=true`, and `status=ok`.

## Local output-cone recovery

```sh
./target/release/guarded-continuation-checker \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,8,12 10,100,1000 100 12 24680 \
  results/reproduced-local-temporal-compositions-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  5,9,13 37,333,2000 100 13 1357911 \
  results/reproduced-local-temporal-compositions-holdout.csv
```

Both grids were fixed before their runs. Every one of the 72 data rows must
report `agreement=true`, `witnesses_valid=true`, and `status=ok`.

## Symbolic local-function replay

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  16,32 10,100,500 50 32 4242001 \
  results/reproduced-symbolic-temporal-compositions-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-symbolic-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  24,48,64 37,333,1000 50 64 9001009 \
  results/reproduced-symbolic-temporal-compositions-holdout.csv
```

Queries fully assign the initial frame and add up to four later observations.
Every admitted row must report agreement and witness validity.

## Exact symbolic preimages

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 2,4,8,16 100 200000 natural 707070 \
  results/reproduced-symbolic-preimages-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  5,7,9 3,7,15,31 100 200000 natural 808181 \
  results/reproduced-symbolic-preimages-holdout.csv
```

Queries contain two to eight observations at arbitrary frames, so the initial
state is generally partial or entirely unspecified. Admitted rows must report
agreement and witness validity; rejected rows must identify the hard node gate.

## Calibration-free preimage ordering

Run the phase grid once for each order in `natural`, `reverse`, `evenodd`, and
`dependency`:

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  7,9 7,15,31 50 200000 dependency 919191 \
  results/reproduced-preimage-order-dependency-phase.csv
```

Then reproduce the preselected dependency-order holdout:

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  6,8,10 5,13,29 50 200000 dependency 20260716 \
  results/reproduced-preimage-order-dependency-holdout.csv
```

## Asymmetric ordering holdout

Run the four phase orders by replacing `ORDER` with `natural`, `reverse`,
`evenodd`, and `dependency`:

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  hub3,tree3,irregular3 \
  7,9 7,15,31 50 200000 ORDER 313131 \
  results/reproduced-asymmetric-order-ORDER-phase.csv
```

Then run the frozen dependency rule and natural control:

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  hub3,tree3,irregular3 \
  6,8,10 5,13,29 50 200000 dependency 414141 \
  results/reproduced-asymmetric-order-dependency-holdout.csv
```

## Exact frame-cycle checkpoints

```sh
./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  6,8 100,1000,10000 50 200000 dependency 515151 \
  results/reproduced-preimage-cycles-phase.csv

./target/release/guarded-continuation-checker \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  5,7,9 137,1333,7777 50 200000 dependency 616161 \
  results/reproduced-preimage-cycles-holdout.csv
```

For admitted rows, `compiled_frames` must be much smaller than the horizon and
`cycle_length` must be positive. Rejected rows must identify the node gate.

## Pre-cycle growth guard

Repeat the cycle phase and holdout commands with `dependency-guard` in place of
`dependency`, seeds unchanged, and outputs
`results/reproduced-preimage-growth-guard-{phase,holdout}.csv`. Admission,
agreement, and witness validity must match the unguarded cohorts. The rejected
holdout rows must identify frame 56 and 192,220 nodes in their status.

## Exact BDD/CDCL hybrid

Repeat the cycle phase and holdout commands with `hybrid` as the order and write
to `results/reproduced-hybrid-preimages-{phase,holdout}.csv`. Phase must report 24
`bdd` rows. Holdout must report 33 `bdd` and three `cdcl-fallback` rows. Every row
must be admitted with `agreement=true` and `witnesses_valid=true`.

## Exact BDD-prefix CDCL checkpoint

Run `benchmark-checkpoint-cdcl cascade4 9 137,1333 50 CHECKPOINT 200000 717171`
for checkpoints 10, 20, and 40, writing separate CSV files. Then reproduce the
preselected holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-checkpoint-cdcl cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-cdcl-10-holdout.csv
```

Every row must agree with the full-CDCL baseline and return valid witnesses.

## Structurally hashed AIG checkpoint

Run the phase and preselected holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-checkpoint-aig cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-checkpoint-aig-10-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-checkpoint-aig cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-aig-10-holdout.csv
```

Every row must report `encoding=aig`, exact agreement, and valid witnesses.

## Lazy observation-cone checkpoint

Run the final direct-root-assumption phase and holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-checkpoint-lazy cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-checkpoint-lazy-root-assumptions-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-checkpoint-lazy cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-lazy-root-assumptions-holdout.csv
```

Every row must report `encoding=lazy-bdd`, exact agreement, and valid witnesses.
Timing is explicitly exploratory; node and clause counts are deterministic.

## Native BDD-theory bridge

Run the pairwise-propagating phase and holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-native-bdd-theory-pairwise-10-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-native-bdd-theory-pairwise-10-holdout.csv
```

Every row must agree with full CDCL and validate every returned witness.

## BDD conflict generalization

Run the generalized-conflict phase and holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-bdd-conflict-generalization-10-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-bdd-conflict-generalization-10-holdout.csv
```

Every row must agree, validate witnesses, and report learned-clause widths no
greater than the checkpoint width.

## Cached BDD conflict explanations

Run the cached-explanation phase and holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-cached-bdd-conflict-explanations-10-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-cached-bdd-conflict-explanations-10-holdout.csv
```

Every row must agree, validate witnesses, and preserve the generalized clause
width and conflict counts recorded by the greedy predecessor.

## Reusable global checkpoint clauses

Run the global-clause phase and preselected holdout with:

```sh
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-global-checkpoint-clauses-10-phase.csv
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-global-checkpoint-clauses-10-holdout.csv
```

Every row must report 134 global clauses with average width 6.28, exact agreement,
valid witnesses, and a finite recognition-inclusive break-even query count.

## Asymmetric cross-family validation

For each `KIND`/`SEED` pair `hub3/919191`, `tree3/929292`, and
`irregular3/939393`, run:

```sh
./target/release/guarded-continuation-checker \
  benchmark-native-bdd-theory KIND 7,9,11 137,1333 50 10 200000 SEED \
  results/reproduced-global-clauses-KIND-generalization.csv
```

All 18 rows must agree and validate witnesses. Six measured rows should report a
speedup above one; timing is exploratory, while logical outcomes and structural
counts are deterministic.

## Calibration-free CQ-SAT/GCC portfolio

Run the accelerated and safe-fallback examples:

```sh
./target/release/guarded-continuation-checker \
  benchmark-cq-portfolio watchdog4 9 137,1333,7777 50 10 200000 4141414 \
  results/reproduced-watchdog-portfolio.csv
./target/release/guarded-continuation-checker \
  benchmark-cq-portfolio sensor-vote3 8,12 257,2049 50 10 200000 5151515 \
  results/reproduced-sensor-vote-portfolio.csv
```

Watchdog rows must select `cq-gcc` for `dense-transition`; sensor rows must select
`cdcl` for `cdcl-fallback`. Every row must agree and validate witnesses. Query
speed for CDCL fallback is normalized to one because it is the baseline path.

The frozen unseen holdouts use `majority3`, `mux3`, and `mixed3` at widths
6, 8, 10, and 12 and horizons 257 and 2,049. All 24 final conservative-gate rows
must select CDCL, agree, and validate witnesses. The rejected broad-gate candidate
and its two 200-query mixed-dynamics stability seeds remain in `results` as the
counterexample that motivated the conservative release rule.

## External AIGER counter-overflow model

```sh
./target/release/guarded-continuation-checker \
  verify-cq-aiger examples/aiger/counter-overflow-4.aag \
  137 10 200000 results/reproduced-aiger-counter.csv \
  results/reproduced-aiger-counter-safety.txt
```

The single row must report `width=4`, `backend=cdcl`,
`gate_reason=cdcl-fallback`, `assumptions_per_query=8`, `queries=137`,
`sat_queries=8`, `unsat_queries=129`, `agreement=true`, and
`witnesses_valid=true`. The safety result must report `status=UNSAFE` and
`bad_frame=15`, followed by the full frame trace. Timing columns are exploratory.
The model's upstream revision and third-party MIT license are recorded under
`examples/aiger`.

## Input-driven AIGER protocol and hardware models

```sh
./target/release/guarded-continuation-checker \
  verify-cq-aiger \
  examples/aiger/petersons-algorithm-2-threads-1-core.aag \
  100 10 200000 results/reproduced-aiger-peterson.csv \
  results/reproduced-aiger-peterson-safety.txt
./target/release/guarded-continuation-checker \
  verify-cq-aiger examples/aiger/spi-bus-receive-e-08-bits.aag \
  50 10 200000 results/reproduced-aiger-spi.csv \
  results/reproduced-aiger-spi-safety.txt
```

The Peterson row must report `variables=12120`, `clauses=34836`, `queries=1`,
`backend=cdcl`, `gate_reason=aiger-primary-inputs`, `sat_queries=0`,
`unsat_queries=1`, and valid agreement/witness fields. Its safety result must be
`SAFE` through horizon 100.

The SPI row must report `variables=4692`, `clauses=12631`, `queries=1`, the same
CDCL gate reason, `sat_queries=1`, `unsat_queries=0`, and valid
agreement/witness fields. Its result must be `UNSAFE` with `bad_frame=16` and a
17-frame latch/input trace. Timing is exploratory.

## SystemVerilog firmware safety gate

Install Yosys and build the release binary, then run both product paths:

```sh
./target/release/guarded-continuation-checker \
  firmware-rtl-safety-gate \
  examples/products/infusion-pump/rtl/safe-controller.sv \
  infusion_pump_controller 8 results/rtl-safe
./target/release/guarded-continuation-checker \
  firmware-rtl-safety-gate \
  examples/products/infusion-pump/rtl/door-interlock-regression.sv \
  infusion_pump_controller 8 results/rtl-regression
```

The first command must exit 0 and report `SAFE`. The second must exit 1, report
`UNSAFE` at `bad_frame=1`, name the `bad` output, and contain this trace:

```text
named_frame,requested_motor_active,motor_request,door_open
0,0,1,0
1,1,0,1
```

Both directories must contain the staged source, synthesis script and logs,
ASCII AIGER model, signal map, metrics, safety report, and final run manifest.
Repository CI independently requires the matching SymbiYosys/Z3 `.sby` jobs to
return PASS for the protected controller and FAIL for the regression.

For the split-source form of the multi-module controller:

```sh
target/release/guarded-continuation-checker \
  firmware-rtl-project-safety-gate infusion_pump_system 8 \
  results/rtl-project-safe \
  examples/products/infusion-pump/rtl/project/pump-components.sv \
  examples/products/infusion-pump/rtl/project/pump-system.sv
```

The command must exit 0, report `SAFE`, publish `source-0000.sv` and
`source-0001.sv`, and record `source_count=2` with ordered `source_0` and
`source_1` entries under `schema_version=4`. Reusing the directory with the legacy single-file command
must remove both numbered snapshots before publishing `source.sv` and the new
manifest. CI independently requires `project/pump-system.sby` to pass through
depth 16 with SymbiYosys and Z3.

Run the versioned project contract with includes, a top parameter, declared
clock/reset policy, and an inferred memory:

```sh
target/release/guarded-continuation-checker \
  firmware-rtl-config-safety-gate \
  examples/products/infusion-pump/rtl/config-project/cq-project.conf \
  results/rtl-config-project
target/release/guarded-continuation-checker \
  firmware-artifact-validate results/rtl-config-project
```

Both commands must exit 0. The synthesis evidence must contain
`-Iinclude-0000`, `chparam -set DEPTH 8`, the declared clock selection, and
`memory_map`. The manifest must record schema 4, CLI 2, `clk:posedge`, the
config-v2 `rst_n:active-low:1` startup policy, and `DEPTH:8`. Frame 0 must hold
`rst_n=0`; every later frame must hold `rst_n=1`. CI and the release procedure also
require `config-project/memory-controller.sby` to pass through depth 8 using the
pinned SymbiYosys revision and Z3.

To verify explicit environment assumptions, run the known-unsafe door-interlock
model with the door-closed contract:

```sh
target/release/guarded-continuation-checker \
  firmware-rtl-constrained-project-safety-gate \
  infusion_pump_controller 8 results/rtl-door-closed \
  examples/products/infusion-pump/rtl/door-closed.assumptions \
  examples/products/infusion-pump/rtl/door-interlock-regression.sv
```

The constrained command must exit 0 and report `SAFE`, with
`assumption_0=door_open=0` in `safety-report.txt`. The same RTL through the
unconstrained gate must exit 1 and report `UNSAFE` at frame 1. CI independently
requires `door-interlock-assumed-safe.sby` to pass through depth 8 and the
unconstrained `door-interlock-regression.sby` to fail. This paired check detects
constraint polarity or scope errors rather than merely confirming one result.

On Linux, every RTL gate report and manifest must contain:

```text
containment_platform=linux
process_group_timeout_kill=true
synthesis_memory_limit_kind=address-space
synthesis_memory_limit_bytes=2147483648
synthesis_file_limit_bytes=536870912
```

The test suite runs three adversarial containment probes: a Python allocation
larger than its reduced address-space limit must fail, a write beyond a reduced
file limit must fail without exceeding the cap, and a shell descendant must no
longer exist after group timeout. macOS runs the latter two probes and records a
zero-byte, `unavailable` memory limit rather than claiming enforcement that the
platform cannot provide reliably.

Validate either completed bundle with:

```sh
target/release/guarded-continuation-checker \
  firmware-artifact-validate results/rtl-project-safe
```

The command must report `VALID` and exit 0. The compatibility test then changes
SAFE to UNSAFE without changing the report and appends an unknown manifest field;
both mutations must be rejected. See
[RTL artifact schema v4](ARTIFACT_SCHEMA_V4.md) for the exact contract.

`cargo test --locked` also runs 5,000 AIGER mutations, 5,000 assumptions-file
mutations, 5,000 project-config mutations, and 10,000 CLI mutations. Seeds live
under `tests/fuzz-corpus`; a
crash or hang-inducing discovery must be minimized and committed there before
the fix is merged.

Confirm the machine-readable compatibility versions with:

```sh
target/release/guarded-continuation-checker firmware-cli-version
```

The exact output is `firmware_cli_version=2 artifact_schema_version=4`. See
[Firmware CLI contract v2](FIRMWARE_CLI_V2.md) for fixed command signatures and
exit meanings.

### Multi-module query-reuse benchmark

Generate the checked-in model from its five-module SystemVerilog source and run
the fixed release-mode grid:

```sh
cd examples/products/infusion-pump/rtl
yosys -Q -q -s synthesize-multimodule.ys
cd ../../../..
cargo build --release
target/release/guarded-continuation-checker \
  benchmark-aiger-query-reuse \
  examples/products/infusion-pump/rtl/multimodule-controller.aag \
  8,16,32,64 10 results/reproduced-rtl-query-reuse.csv
```

Each row must contain four distinct properties, two-property reuse batches,
exact reusable/cold agreement, and `status=ok`. The static selector must choose
`bounded-reuse` through horizon 16 and `cold-bmc` at horizons 32 and 64. Timing ratios
are exploratory and machine-dependent; the committed reference run is
`results/rtl-multimodule-query-reuse-v1.csv`. Repository CI also requires the
independent `multimodule-controller.sby` SymbiYosys/Z3 job to pass through depth
16.

## Curated result files

### Public RTL compatibility corpus

Build a release binary, install Yosys, Z3, and the pinned SymbiYosys revision
documented by CI, then run:

```sh
scripts/run-rtl-corpus.sh \
  target/release/guarded-continuation-checker \
  corpus/rtl/yosys-simple \
  results/reproduced-rtl-public-corpus \
  /path/to/sby/sbysrc/sby.py
```

The runner first verifies every upstream SHA-256 digest. It then requires each
CQ result and exit status to match the strict manifest, validates every emitted
evidence bundle, and requires the independent SymbiYosys/Z3 result to agree.
The output is written atomically to `results.csv`; any missing, malformed, or
disagreeing case fails the run.

CI repeats the CQ checks inside the digest-pinned historical image
`hdlc/yosys@sha256:58c0c80e41fd96b4b90da53c730aa3c43051f0cf2a6c6e336bd012281479df22`
(Yosys 0.36+42). The independent oracle is intentionally run only on the current
toolchain because the historical pass is a synthesis-compatibility check. The
committed reference summaries are
`results/rtl-public-corpus-yosys-067-v1.csv` and
`results/rtl-public-corpus-yosys-036-v1.csv`.

Each CSV in `results` is a compact summary. Seeds, cohort sizes, admission,
agreement, and witness-validity columns are part of the experimental contract.

### Proof-carrying controller batch benchmark

Compare repeated public composition calls with the verified-controller batch
path in an optimised build:

```sh
cargo run --release --example controller_plant_batch_benchmark
```

The benchmark uses 101 interleaved trials at batch sizes 1 through 64, requires
exact agreement for every member, and reports both median checking time and
complete canonical artifact byte ratios. The baseline carries one independently
verified controller-plant artifact per member. The shared path carries one
controller transducer plus all source-bound member results.

### Public controller MTBDD reuse benchmark

Run the complete-artifact baseline for the pinned public washing controller:

```sh
cargo run --release --example public_washing_controller_mtbdd_reuse_benchmark
```

The benchmark uses three interleaved trials for 1, 2, 4, 8, and 16 appliance
monitor members. It requires every complete shared result to equal the repeated
independently verified result and reports encoded bytes plus median release-mode
checking time. The committed reference is
`results/public-washing-controller-mtbdd-reuse-v1.csv`.

Cross-check the two retained public-controller composition answers with a
SymbiYosys checkout at revision
`fea6e467d067b3ea84b6b5ac08cd48beb59f0d42`, maintained Yosys, and Z3:

```sh
scripts/test-public-washing-controller-oracle.sh /path/to/sby.py
```

The expected result is one depth-32 PASS and one step-10 FAIL, recorded in
`results/public-washing-controller-mtbdd-oracle-v1.csv`. The script verifies
the pinned inputs and regenerates the AIGER file byte for byte before checking.

### Stateful physical-plant three-way baseline

Regenerate the repository-authored physical plant and verify its digests:

```sh
cd corpus/rtl/wmcontroller/plant
shasum -a 256 -c SHA256SUMS
yosys -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
cd ../../../..
```

Run the six-property shared, repeated, and checked in-process comparison:

```sh
cargo run --release --locked \
  --example public_washing_controller_physical_plant_benchmark
```

Every run must report two SAFE and four UNSAFE answers, exact agreement, and
`status=ok`. Timing values are observations and are expected to vary.

Cross-check all six properties in one compiled SymbiYosys and Z3 session:

```sh
scripts/test-public-washing-controller-physical-oracle.sh /path/to/sby.py
```

The script regenerates both generated AIGER inputs byte for byte, requires the
four exact bad frames, and checks that both expected-SAFE assertions survive
through frame 32. The committed reference is
`results/public-washing-controller-physical-oracle-v1.csv`.

Regenerate and check the seven-action complete-cycle plant, then run its exact
composition regression:

```sh
cd corpus/rtl/wmcontroller/physical-plant
shasum -a 256 -c SHA256SUMS
yosys -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
cd ../../../..
cargo test --locked --test public_washing_controller_physical_plant_api
```

### Controller MTBDD self-service acceptance

```sh
cargo build --release --locked
scripts/run-controller-mtbdd-self-service-acceptance.sh \
  target/release/guarded-continuation-checker \
  target/controller-mtbdd-acceptance.csv
diff -u results/controller-mtbdd-self-service-acceptance-v1.csv \
  target/controller-mtbdd-acceptance.csv
```

The harness requires exact answers and bad frames in a fresh verifier process,
then rejects manifest drift and artifact mutation.

Exercise the same contract through the typed, shell-free Rust API:

```sh
cargo test --locked --test controller_mtbdd_tool_api
```

This test discovers capabilities, produces and verifies a two-member artifact,
checks ordered SAFE and UNSAFE results, and records the operation and success
status exposed by invocation metrics schema v1.

### Controller MTBDD whole-process resources

With the pinned SymbiYosys checkout and an optimised GCC binary:

```sh
scripts/benchmark-controller-mtbdd-process-resources.sh \
  target/release/guarded-continuation-checker /path/to/sby.py \
  /tmp/controller-resources.csv
```

The no-overwrite CSV records wall time, peak RSS, timing backend and platform
for GCC artifact production, fresh verification and the maintained formal
oracle. `TRIALS` defaults to 3 and is bounded to 100. Exact timings are host
observations and must not be used as portfolio inputs. The retained arm64
reference is `results/controller-mtbdd-process-resources-v1.csv`.

### Controller plant exact portfolio acceptance

```sh
cargo build --release --locked
scripts/run-controller-plant-portfolio-acceptance.sh \
  target/release/guarded-continuation-checker \
  target/controller-plant-portfolio.csv
diff -u results/controller-plant-portfolio-acceptance-v1.csv \
  target/controller-plant-portfolio.csv
```

The harness requires the public six-member batch to take the MTBDD route and a
nine-action boundary fixture to take direct exact fallback. It preserves both
answers, then rejects mutation and output collision. Unit and downstream tests
add reason-tampering, downgrade, truncation, and complete mutation coverage.

Measure portfolio phase attribution without changing static routing:

```sh
cargo build --release --locked
TRIALS=3 scripts/benchmark-controller-plant-portfolio-phases.sh \
  target/release/guarded-continuation-checker \
  target/controller-plant-portfolio-phases.csv
```

The retained host observations are
`results/controller-plant-portfolio-phases-v1.csv`. Timing rows vary by host
and are exercised, but not compared byte for byte, in CI.

### Proof-carrying MTBDD plant verification

```sh
scripts/benchmark-controller-proof-mtbdd-plant.sh \
  target/controller-proof-mtbdd-plant.csv
```

The benchmark compares the compact exhaustive verifier with the proof-carrying
verifier on the same six-member public physical-plant batch. It requires exact
member-result agreement and reports artifact size, production cost, and median
verification time across three trials. The retained arm64 observation is
`results/public-washing-controller-proof-mtbdd-plant-v1.csv`. Timings vary by
host and are evidence, not an acceptance threshold.
