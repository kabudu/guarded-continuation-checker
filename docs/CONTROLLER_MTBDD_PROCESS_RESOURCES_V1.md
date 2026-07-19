# Controller MTBDD process-resource baseline v1

Status: measured engineering baseline, not a performance superiority claim.

## Question

What wall time and peak resident memory do the self-service GCC producer and
verifier consume relative to the maintained SymbiYosys, Yosys, and Z3 physical
plant oracle?

The comparison deliberately reports three separate processes. Their scopes are
not equivalent:

- `gcc_produce` exhaustively verifies the controller function, checks all six
  closed-loop properties, and creates source-bound reusable evidence.
- `gcc_verify` independently checks that evidence, the complete controller
  function, manifest bindings, and all six property results.
- `symbiyosys_oracle` regenerates the two AIGER models and checks the six
  closed-loop properties through frame 32. It does not create or check GCC's
  reusable controller evidence contract.

## Retained observation

Three interleaved macOS trials used Rust 1.97.0 release builds and the pinned
SymbiYosys revision. Median results were:

| Process | Median time | Median peak RSS | Time / oracle | RSS / oracle |
| --- | ---: | ---: | ---: | ---: |
| GCC produce | 2.46 s | 17,285,120 B | 6.83 | 0.590 |
| GCC verify | 0.97 s | 4,292,608 B | 2.69 | 0.147 |
| SymbiYosys oracle | 0.36 s | 29,278,208 B | 1.00 | 1.000 |

The result rejects a speed-win claim on this small public batch. It supports a
narrow peak-memory observation, especially for independent verification. Since
the checked scopes differ, neither ratio is an end-to-end solver comparison.
The raw observations are retained in
`results/controller-mtbdd-process-resources-v1.csv`.

## Reproduce

```sh
cargo build --release --locked
TRIALS=3 scripts/benchmark-controller-mtbdd-process-resources.sh \
  target/release/guarded-continuation-checker \
  /path/to/pinned/sbysrc/sby.py \
  target/controller-mtbdd-process-resources.csv
```

The harness supports BSD `time -l` on macOS and GNU `time` on Linux. Absolute
values vary by host, workload, and toolchain. Compare medians within one
interleaved run rather than comparing the retained macOS values to another
machine.
