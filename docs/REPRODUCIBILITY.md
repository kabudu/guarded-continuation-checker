# Reproducibility

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

## Modular DIMACS result

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-dimacs \
  examples/modular-demo.cnf 10000 10 results/reproduced-modular.csv
```

## SATLIB gate coverage

Obtain SATLIB instances from an authorized source and run the DIMACS command for
each local file. Third-party formulas are not redistributed in this repository.
The curated aggregate result is `results/continuation-dimacs-summary-v1.csv`.

## Scaling

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-quotients \
  banded-planted 100 4 98302 1 results/reproduced-scaling.csv
```

## Repeated queries

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-reuse \
  banded-planted 100 4 98302 20000 results/reproduced-reuse.csv
```

## Wide assumptions

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-reuse-stress \
  banded-planted 100 4 98302 20000 40 results/reproduced-stress.csv
```

## Update repairs

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-repairs \
  banded-planted 100 4 98302 20 200 results/reproduced-repairs.csv
```

Deletion rows intentionally rebuild from layer zero. Do not interpret the older
v1 deletion summary from the historical experiment repository as a valid generic
speed result; `continuation-repairs-summary-v2.csv` is authoritative.

## Temporal bounded-width phase

```sh
./target/release/continuation-quotient-sat \
  benchmark-continuation-temporal-phase \
  2,4,6 10,100,1000,10000 100 12 424242 \
  results/reproduced-temporal-long.csv

./target/release/continuation-quotient-sat \
  benchmark-continuation-temporal-phase \
  8 10,100,1000 100 12 424242 \
  results/reproduced-temporal-width8.csv

./target/release/continuation-quotient-sat \
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
./target/release/continuation-quotient-sat \
  benchmark-temporal-vocabulary \
  copy,negate,permute,xor,circuit \
  4,6,8 10,100,1000 100 8 777 \
  results/reproduced-temporal-vocabulary-phase.csv

./target/release/continuation-quotient-sat \
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
./target/release/continuation-quotient-sat \
  benchmark-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 10,100,1000 100 8 12345 \
  results/reproduced-temporal-compositions-phase.csv

./target/release/continuation-quotient-sat \
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
./target/release/continuation-quotient-sat \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  4,8,12 10,100,1000 100 12 24680 \
  results/reproduced-local-temporal-compositions-phase.csv

./target/release/continuation-quotient-sat \
  benchmark-local-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  5,9,13 37,333,2000 100 13 1357911 \
  results/reproduced-local-temporal-compositions-holdout.csv
```

Both grids were fixed before their runs. Every one of the 72 data rows must
report `agreement=true`, `witnesses_valid=true`, and `status=ok`.

## Symbolic local-function replay

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  16,32 10,100,500 50 32 4242001 \
  results/reproduced-symbolic-temporal-compositions-phase.csv

./target/release/continuation-quotient-sat \
  benchmark-symbolic-temporal-compositions \
  majority3,mux3,mixed3,cascade4 \
  24,48,64 37,333,1000 50 64 9001009 \
  results/reproduced-symbolic-temporal-compositions-holdout.csv
```

Queries fully assign the initial frame and add up to four later observations.
Every admitted row must report agreement and witness validity.

## Exact symbolic preimages

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  4,6,8 2,4,8,16 100 200000 natural 707070 \
  results/reproduced-symbolic-preimages-phase.csv

./target/release/continuation-quotient-sat \
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
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  7,9 7,15,31 50 200000 dependency 919191 \
  results/reproduced-preimage-order-dependency-phase.csv
```

Then reproduce the preselected dependency-order holdout:

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  6,8,10 5,13,29 50 200000 dependency 20260716 \
  results/reproduced-preimage-order-dependency-holdout.csv
```

## Asymmetric ordering holdout

Run the four phase orders by replacing `ORDER` with `natural`, `reverse`,
`evenodd`, and `dependency`:

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  hub3,tree3,irregular3 \
  7,9 7,15,31 50 200000 ORDER 313131 \
  results/reproduced-asymmetric-order-ORDER-phase.csv
```

Then run the frozen dependency rule and natural control:

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  hub3,tree3,irregular3 \
  6,8,10 5,13,29 50 200000 dependency 414141 \
  results/reproduced-asymmetric-order-dependency-holdout.csv
```

## Exact frame-cycle checkpoints

```sh
./target/release/continuation-quotient-sat \
  benchmark-symbolic-preimages \
  majority3,mux3,mixed3,cascade4 \
  6,8 100,1000,10000 50 200000 dependency 515151 \
  results/reproduced-preimage-cycles-phase.csv

./target/release/continuation-quotient-sat \
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
./target/release/continuation-quotient-sat \
  benchmark-checkpoint-cdcl cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-cdcl-10-holdout.csv
```

Every row must agree with the full-CDCL baseline and return valid witnesses.

## Structurally hashed AIG checkpoint

Run the phase and preselected holdout with:

```sh
./target/release/continuation-quotient-sat \
  benchmark-checkpoint-aig cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-checkpoint-aig-10-phase.csv
./target/release/continuation-quotient-sat \
  benchmark-checkpoint-aig cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-aig-10-holdout.csv
```

Every row must report `encoding=aig`, exact agreement, and valid witnesses.

## Lazy observation-cone checkpoint

Run the final direct-root-assumption phase and holdout with:

```sh
./target/release/continuation-quotient-sat \
  benchmark-checkpoint-lazy cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-checkpoint-lazy-root-assumptions-phase.csv
./target/release/continuation-quotient-sat \
  benchmark-checkpoint-lazy cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-checkpoint-lazy-root-assumptions-holdout.csv
```

Every row must report `encoding=lazy-bdd`, exact agreement, and valid witnesses.
Timing is explicitly exploratory; node and clause counts are deterministic.

## Native BDD-theory bridge

Run the pairwise-propagating phase and holdout with:

```sh
./target/release/continuation-quotient-sat \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-native-bdd-theory-pairwise-10-phase.csv
./target/release/continuation-quotient-sat \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-native-bdd-theory-pairwise-10-holdout.csv
```

Every row must agree with full CDCL and validate every returned witness.

## BDD conflict generalization

Run the generalized-conflict phase and holdout with:

```sh
./target/release/continuation-quotient-sat \
  benchmark-native-bdd-theory cascade4 9 137,1333 50 10 200000 717171 \
  results/reproduced-bdd-conflict-generalization-10-phase.csv
./target/release/continuation-quotient-sat \
  benchmark-native-bdd-theory cascade4 9 7777 50 10 200000 818181 \
  results/reproduced-bdd-conflict-generalization-10-holdout.csv
```

Every row must agree, validate witnesses, and report learned-clause widths no
greater than the checkpoint width.

## Curated result files

Each CSV in `results` is a compact summary. Seeds, cohort sizes, admission,
agreement, and witness-validity columns are part of the experimental contract.
