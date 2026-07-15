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

## Curated result files

Each CSV in `results` is a compact summary. Seeds, cohort sizes, admission,
agreement, and witness-validity columns are part of the experimental contract.
