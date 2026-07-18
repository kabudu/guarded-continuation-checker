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
guarded-continuation-checker benchmark-aiger-predicate-certificate-cost \
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

The follow-up [proof-carrying relation experiment](PREDICATE_PROOF_RELATION_EXPERIMENT.md)
achieves that target for one-step relation reconstruction, including a 280.32x
checker speedup at 16 inputs. It is integrated into certificate v2 below.

## Canonical certificate v2 result

`benchmark-aiger-predicate-certificate-v2-cost` measures end-to-end production
and independent verification of the canonical v2 artifact. The verifier rebuilds
the CNF obligations, checks every native proof with `varisat-checker`, evaluates
the concrete witnesses directly, and recomputes relation powers, composition and
the final answer without invoking the BDD producer.

| Model/query | Result | V2 generation | V2 verification | CDCL query | V2 artifact | Raw proofs | Proofs | Direct evaluations | V2 check/CDCL |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Interrupt, 9 inputs, h8 | avoidable | 33.273 ms | 0.308 ms | 0.212 ms | 9,706 B | 4,237 B | 5 | 29 | 1.46x |
| Actuator, 12 inputs, h16 | avoidable | 37.872 ms | 0.348 ms | 0.253 ms | 14,626 B | 5,954 B | 9 | 89 | 1.38x |
| Sensor fusion, 16 inputs, h32 | avoidable | 38.777 ms | 0.831 ms | 0.548 ms | 52,119 B | 21,853 B | 17 | 305 | 1.52x |
| Actuator, 12 inputs, h1 | unavoidable | 34.947 ms | 0.273 ms | 0.046 ms | 10,851 B | 4,788 B | 9 | 15 | 5.89x |

Raw v2 evidence:

- [`predicate-certificate-v2-cost-interrupt-h8-v1.csv`](../results/predicate-certificate-v2-cost-interrupt-h8-v1.csv)
- [`predicate-certificate-v2-cost-actuator-h16-v1.csv`](../results/predicate-certificate-v2-cost-actuator-h16-v1.csv)
- [`predicate-certificate-v2-cost-sensor-h32-v1.csv`](../results/predicate-certificate-v2-cost-sensor-h32-v1.csv)
- [`predicate-certificate-v2-cost-actuator-h1-unavoidable-v1.csv`](../results/predicate-certificate-v2-cost-actuator-h1-unavoidable-v1.csv)

V2 removes the v1 verifier's exponential input enumeration: at 16 inputs the
median check falls from 136.045 ms to 0.831 ms, a 163.71x end-to-end reduction.
It also makes the evidence substantially larger and production about three times
slower than v1. Verification remains 1.38–1.52x CDCL on the three avoidable rows
and 5.89x on the short unavoidable row. This is a successful checker-complexity
result, not a claim that proof-carrying queries beat CDCL or that v2 is ready to
replace the portfolio default.

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

The complete answer-balanced v1 and v2 cohorts can be regenerated into a new
directory:

```sh
./scripts/benchmark-predicate-certificate-cost.sh /tmp/cq-certificate-cost 10
```
