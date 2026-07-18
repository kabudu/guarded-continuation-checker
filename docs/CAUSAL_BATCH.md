# Compile-once causal batch experiment

This experiment asks whether CQ-SAT/GCC's preparation cost can be amortised by
explaining many bounded failures from one sequential design. It is an exact,
bounded research benchmark—not a claim of a new causal definition or a
production performance result.

## Workload

For one AIGER model and maximum horizon, the command:

1. builds the transition CNF once;
2. compiles at most one continuation quotient from that target-independent CNF;
3. tests every `(frame, bad-output)` query and retains every reachable failure;
4. derives a counterexample observation vocabulary for each failure;
5. enumerates up to the requested number of distinct 1-minimal sufficient
   causes with a monotone subset map; and
6. replays the identical enumeration transcript through one persistent Varisat
   instance and the shared continuation quotient.

The causal oracle asks whether the observed input values make the negation of
the bad output unsatisfiable. A cause is therefore sufficient when every trace
consistent with those observations fails at that target. It is 1-minimal when
removing any one selected observation permits a non-failing trace.

Three semantics-preserving vocabularies are tested:

- `segments`: maximal constant runs for each primary input;
- `points`: one observation per input and frame; and
- `dyadic`: every aligned, power-of-two constant interval, including point
  leaves.

Dyadic observations may overlap; overlapping observations always carry the
same witness value. Protocol-specific names and transition relations are not
inferred from geometry. Such vocabularies require explicit design metadata or
a future relational-observation schema.

## Run and verify

```sh
cargo build --release --locked

target/release/continuation-quotient-sat \
  benchmark-aiger-causal-batch \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  8 16 8 100 target/causal-batch.csv

target/release/continuation-quotient-sat \
  verify-aiger-causal-batch \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  target/causal-batch.csv
```

The input may be original-format ASCII `.aag` or binary `.aig`. The benchmark
refuses to overwrite its output and publishes through the existing atomic
no-clobber path. The verifier checks the schema and source SHA-256, reconstructs
the target witnesses and vocabularies, and independently proves every published
cause sufficient and 1-minimal with fresh CDCL.

## Bounds and interpretation

- maximum 256 `(frame, output)` targets;
- maximum 512 observations per vocabulary;
- maximum 16 reported causes per row;
- maximum 4,096 subset-map iterations;
- maximum 10,000 transcript repetitions;
- existing 250-million conservative query-work limit; and
- existing CQ admission limits of 256 CNF variables, 4,096 clauses, and at most
  20 requested frontier bits.

`enumeration_complete=true` means the subset map was exhausted before the
cause limit. `false` is an honest truncated result, not evidence that no other
minimal causes exist.

`workload_measured_break_even_query` is the first replayed oracle query across
the cumulative target/vocabulary workload at which CQ
preparation plus cumulative CQ query time is no greater than persistent-CDCL
setup plus cumulative query time. `workload_projected_break_even_query` extrapolates
from the cumulative observed per-query averages only when CQ has a positive
average query-time advantage. Each row reports the workload state after that
row. `none` means no crossover was observed or supported by the measured
averages. Timing fields are performance observations; semantic agreement and
independently replayed minimality are the correctness claims.

On the first controlled infusion-pump run at horizon 8 with 100 replays, all 24
target/vocabulary rows completed: eight reachable failure targets, 35 minimal
causes, and 619 discovery-oracle queries. Every cause passed independent replay.
CQ was admitted, but neither an observed nor projected workload crossover was
found. This negative result says that compile-once reuse alone is not yet enough
for this small design and query volume.
