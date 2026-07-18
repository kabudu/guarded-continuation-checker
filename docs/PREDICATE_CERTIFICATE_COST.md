# Predicate certificate cost experiment

This experiment measures the operational cost of proof-carrying dense predicate
composition. It is deliberately answer-balanced and preserves all ten release
trials per row. It does not claim that certificate production is faster than
SAT solving.

## Question

For the identical AIGER model, terminal bad output and partial input transcript:

1. how much does a fresh predicate compile-and-query cost;
2. how much does full certificate generation and atomic publication cost;
3. how much does the independent exhaustive checker cost;
4. how much does a fresh exact persistent-CDCL query path cost; and
5. do all three logical answers agree?

## Frozen schema

`benchmark-aiger-predicate-certificate-cost` emits schema v1 with one row per
trial. The source and transcript have separate SHA-256 bindings. The four timing
columns, certificate byte count, result, agreement and status are never
aggregated or filtered by the producer.

```sh
continuation-quotient-sat benchmark-aiger-predicate-certificate-cost \
  INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt REPEATS OUTPUT.csv
```

The command accepts 1–1,000 repeats, refuses to overwrite its output, uses a
fresh predicate quotient, certificate and CDCL solver for every trial, requires
all answers to agree, removes each temporary certificate and publishes the CSV
atomically.

## Measurement boundary

The columns are intentionally separate because they have different boundaries:

- `predicate_query_ns` includes fresh BDD predicate compilation and the query,
  using the already parsed model and transcript;
- `certificate_generate_ns` is end-to-end certificate production and includes
  source/transcript parsing, BDD work, hashing, canonical serialisation, file
  creation, syncing and atomic publication;
- `certificate_verify_ns` is end-to-end independent checking and includes
  parsing, hashing, support recovery and exhaustive direct-AIG evaluation;
- `cdcl_query_ns` includes bounded CNF construction and a fresh Varisat solver,
  using the already parsed model and transcript.

Consequently, `predicate_query_ns` and `cdcl_query_ns` are the closest query-core
comparison. Certificate generation and verification are evidence overhead, not
solver speedups. Filesystem timings are host-dependent.

## Release result

Release-mode measurements were taken on 18 July 2026 on the repository host.
Values below are medians of ten raw trials; nanoseconds are retained in the
linked CSVs.

| Model/query | Result | Predicate query | Certificate generation | Independent check | CDCL query | Certificate | CDCL/predicate | Evidence/CDCL |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Interrupt, 9 inputs, h8 | avoidable | 0.116 ms | 10.345 ms | 0.519 ms | 0.331 ms | 546 B | 2.84x | 32.85x |
| Actuator, 12 inputs, h16 | avoidable | 0.259 ms | 12.054 ms | 3.674 ms | 0.266 ms | 650 B | 1.03x | 59.11x |
| Sensor fusion, 16 inputs, h32 | avoidable | 0.484 ms | 13.111 ms | 136.045 ms | 0.580 ms | 840 B | 1.20x | 257.31x |
| Actuator, 12 inputs, h1 | unavoidable | 0.209 ms | 11.806 ms | 0.297 ms | 0.054 ms | 584 B | 0.26x | 225.60x |

Raw evidence:

- [`predicate-certificate-cost-interrupt-h8-v1.csv`](../results/predicate-certificate-cost-interrupt-h8-v1.csv)
- [`predicate-certificate-cost-actuator-h16-v1.csv`](../results/predicate-certificate-cost-actuator-h16-v1.csv)
- [`predicate-certificate-cost-sensor-h32-v1.csv`](../results/predicate-certificate-cost-sensor-h32-v1.csv)
- [`predicate-certificate-cost-actuator-h1-unavoidable-v1.csv`](../results/predicate-certificate-cost-actuator-h1-unavoidable-v1.csv)

## Interpretation

The raw predicate query is competitive on the three statically admitted
avoidable cases, but loses on the short unavoidable control. Proof-carrying
operation is not currently a performance optimisation: atomic certificate
publication dominates narrow cases, and exhaustive checking rises from 0.519 ms
at 9 inputs to 136.045 ms at 16 inputs. Certificate size stays below 1 KiB.

This falsifies any claim that certificate v1 is already cheap enough for every
query. Its present value is independently checkable evidence. The next technical
target is a checker obligation that avoids enumerating every projected input
while retaining a smaller trusted base than the BDD producer.

## Closest-tool boundary

These results compare only CQ-SAT/GCC with its Varisat control. They do not close
the novelty gate requiring experimental comparison with Certifaiger/k-witness,
interactive BDD certification, or proof-producing SAT/QBF systems. Those tools
prove different property classes and cannot be represented as runtime columns
without a documented translation and identical obligations. The
[`novelty gap register`](NOVELTY_GAP.md) therefore remains open.

## Reproduction

The four transcripts are under
[`examples/predicate-certificate-cost`](../examples/predicate-certificate-cost/).
Build with `cargo build --release`, then run the command above for each model and
matching transcript with ten repeats. Output publication is no-overwrite; remove
or rename an existing result deliberately before regeneration.

The complete answer-balanced cohort can be regenerated into a new directory:

```sh
./scripts/benchmark-predicate-certificate-cost.sh /tmp/cq-certificate-cost 10
```
