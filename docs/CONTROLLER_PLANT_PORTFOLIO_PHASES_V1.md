# Controller plant portfolio phase baseline v1

Status: local diagnostic evidence from one arm64 macOS host. These measurements
are observations, never portfolio inputs.

## Question

Where does the self-service portfolio spend time after manifest loading is
separated from artifact production or reading, independent verification, and
create-new output writing?

The harness runs three release-build trials for both retained routes:

- admitted: the six-property stateful washing plant using MTBDD; and
- fallback: the two-property nine-action boundary fixture using direct exact
  evaluation.

The fixtures have different size and complexity. Their absolute times must not
be used as a backend comparison.

## Retained medians

| Route and operation | Load | Artifact | Verification | Write | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| MTBDD produce | 417 us | 1,452,820 us | 971,710 us | 4,601 us | 2,430,709 us |
| MTBDD verify | 418 us | 25 us | 962,823 us | 0 us | 963,261 us |
| Direct fallback produce | 166 us | 138 us | 135 us | 5,300 us | 5,764 us |
| Direct fallback verify | 168 us | 19 us | 139 us | 0 us | 328 us |

For the admitted fixture, MTBDD construction accounts for about 59.8% of the
median create-and-check operation and independent verification about 40.0%.
Fresh verification is almost entirely semantic verification rather than file
loading. For the tiny fallback fixture, output creation dominates production;
the values are too small and fixture-specific for a general performance claim.

## Reproduction

```sh
cargo build --release --locked
scripts/benchmark-controller-plant-portfolio-phases.sh \
  target/release/guarded-continuation-checker \
  target/controller-plant-portfolio-phases.csv
```

`TRIALS` defaults to 3 and is statically limited to 100. The script refuses to
overwrite its output. The retained raw rows are in
`results/controller-plant-portfolio-phases-v1.csv`.

## Claim boundary

This instrumentation identifies optimisation targets. It does not show a
speed advantage, cross-platform stability, or novelty. The next performance
experiment should replicate the phases on hosted Linux and compare an
equivalent non-GCC proof-carrying consumer on the same query and evidence scope.
