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

## Curated result files

Each CSV in `results` is a compact summary. Seeds, cohort sizes, admission,
agreement, and witness-validity columns are part of the experimental contract.
