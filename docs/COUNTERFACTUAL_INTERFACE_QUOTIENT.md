# Counterfactual Interface Quotient

The Counterfactual Interface Quotient (CIQ) is an exact bounded experiment for
firmware controllers with small persistent state and long, repeated input
phases. It answers the causal oracle used by CQ-SAT/GCC:

> Under these observed input values, does any execution remain in which the
> selected bad output is false at the target frame?

CIQ does not compile the complete time-unrolled CNF. It evaluates the AIG once
to construct the finite interface

```text
(latch state, primary inputs) -> (next latch state, bad-output mask)
```

and represents each input-constrained phase as a Boolean relation between
entering and leaving latch states. Equal consecutive input constraints remain
one interval. A length-N interval is composed from cached powers of its
one-frame relation, so repeated phases are handled by exact repeated squaring
rather than frame expansion. Different intervals are composed in temporal
order. A balanced relation tree is retained for on-demand reconstruction of a
complete state/input trace.

## Exactness invariants

- Every relation pair has at least one concrete input/state execution.
- Relation composition existentially quantifies only the shared boundary state.
- An observation update propagates only when it changes the semantic relation.
- Decision queries and persistent CDCL replay the identical causal transcript.
- One avoiding witness per result is reconstructed by each backend and checked
  against the observations, transition table, bad output, and original CNF.
- Any disagreement or invalid witness aborts publication.

The current fail-closed envelope is one to eight latches, one to eight primary
inputs, at most 64 frames, at most 128 bad outputs, at most 1,048,576 explicit
state/input cells, and the existing bounded causal work limits. Designs outside
that envelope remain on persistent CDCL. There is no approximation or
per-formula calibration.

## Run

```sh
cargo build --release --locked

target/release/continuation-quotient-sat \
  benchmark-aiger-interface-quotient \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  64 8 100 target/interface-quotient.csv

target/release/continuation-quotient-sat \
  verify-aiger-interface-quotient \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  target/interface-quotient.csv
```

Arguments are `INPUT HORIZON MAX_CAUSES REPEATS OUTPUT.csv`. Original-format
ASCII and binary AIGER are accepted. The strict schema refuses overwrite and is
published atomically. The separate verifier ignores recorded timings,
re-enumerates every causal query with fresh CDCL, replays CIQ, validates an
independently reconstructed witness, checks the source digest, and rejects
duplicate, altered, or omitted reachable targets.

`interface_query_speedup` compares decision-only causal queries. Both backends
recover one concrete witness separately; their measured recovery costs are
included in `workload_total_speedup`. The workload total charges CIQ interface
compilation and every per-target tree, and charges one persistent-CDCL setup.

Regenerate the four-horizon, ten-trial evidence bundle with:

```sh
scripts/run-interface-quotient-scaling.sh target/interface-quotient-scaling
```

The harness refuses to overwrite an existing bundle and independently verifies
every raw report before producing `summary.csv`. `TRIALS` and `REPEATS` may be
set explicitly for bounded smoke runs.

## First controlled result

The checked-in
[`interface-quotient-scaling-v1.csv`](../results/interface-quotient-scaling-v1.csv)
summarises ten independent arm64 macOS trials per horizon on the infusion-pump
door-interlock regression, with 100 identical transcript replays per target.

| Horizon | Minimum | Median | Maximum |
| ---: | ---: | ---: | ---: |
| 8 | 1.39x | 1.42x | 1.47x |
| 16 | 2.30x | 2.38x | 2.41x |
| 32 | 4.07x | 4.15x | 4.68x |
| 64 | 6.95x | 7.00x | 7.17x |

All queries agreed and all recovered witnesses validated. At horizon 64, the
last target reused powered summaries 2,031 times while constructing only 43
distinct `(input constraint, interval length)` summaries.

This is the first robust positive scaling result in the causal research line.
It is still one deliberately small product-shaped controller, not evidence of
general superiority or production readiness.

## Prior-art and novelty boundary

Finite-state transition relations, repeated squaring, binary lifting, BDD-style
relational products, bounded model checking, incremental SAT, counterexample
minimisation, and projected knowledge compilation are established techniques.
Recent work also provides proof frameworks for certifying projected knowledge
compilation outputs:

- [Certifying Projected Knowledge Compilation, SAT 2025](https://doi.org/10.4230/LIPIcs.SAT.2025.8)
- [Separating Incremental and Non-Incremental Bottom-Up Compilation, SAT 2023](https://doi.org/10.4230/LIPIcs.SAT.2023.7)

The potentially distinctive hypothesis is their exact integration into a
phase-preserving counterfactual interface quotient with semantic no-op repair,
causal enumeration, cross-backend transcript replay, and on-demand trace
reconstruction. The repository has not established scholarly novelty. A formal
closest-method review, independent implementation comparison, and expert review
remain required before making such a claim.

## Falsified variants and next boundary

- Expanding every long observation interval into frame leaves loses more as the
  horizon grows: 0.795x at horizon 8 and 0.714x at horizon 64 in the controlled
  run.
- Caching tiny leaf relations adds hashing and cloning overhead and regresses
  the result.
- Raising explicit input enumeration to 16 inputs produces only 0.06x–0.13x on
  the controlled sparse/dense fixtures.

The next technical boundary is therefore symbolic input projection: preserve a
compact input predicate at the interface instead of enumerating all input
patterns. That is required for wider firmware controllers and future robotics
workloads with sensor vectors, modes, and actuator constraints.
