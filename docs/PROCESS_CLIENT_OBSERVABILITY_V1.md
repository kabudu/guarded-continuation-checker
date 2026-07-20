# Process-client observability v1

Status: stable candidate Rust aggregation API. Not an external monitoring
service or a production-readiness claim.

## Purpose

Every observed typed process operation already returns `InvocationMetrics`,
including failures. `aggregate_invocation_metrics` converts a bounded set of
those rows into one canonical pipeline summary without dropping refusals or
tool errors.

The aggregate records:

- total, successful, and failed jobs;
- total and maximum wall-clock duration;
- total stdout and stderr bytes;
- process-group-contained and memory-limited job counts;
- a deterministic count for every observed operation; and
- a deterministic count for every failure class.

All totals use checked arithmetic. Empty inputs, unknown metrics schemas, more
than 1,000,000 jobs, and arithmetic overflow are rejected. Operation and
failure maps use lexical ordering, so the single-row CSV representation is
canonical for a fixed set of measurements.

## Rust use

Call each typed client's `*_observed` method and retain the metrics whether the
operation succeeds or fails:

```rust,no_run
use guarded_continuation_checker::{
    InvocationMetrics, aggregate_invocation_metrics,
};

fn publish_pipeline_metrics(
    rows: &[InvocationMetrics],
) -> Result<String, Box<dyn std::error::Error>> {
    let aggregate = aggregate_invocation_metrics(rows.iter())?;
    Ok(format!(
        "{}\n{}\n",
        guarded_continuation_checker::InvocationMetricsAggregate::csv_header(),
        aggregate.to_csv_row(),
    ))
}
```

For a failed call, use `PredicateOperationError.metrics` before classifying or
returning its `error`. A resource refusal must therefore increase both the job
count and `resource_refusal` failure count. It must not be removed from the
denominator and must not be converted into a logical result.

## Real workflow evidence

`tests/controller_proof_mtbdd_cli.rs` runs the real governed split executable
and aggregates three process jobs:

1. successful resource-contract discovery;
2. successful two-batch verification; and
3. one deliberate controller-byte refusal.

On Unix, the test requires two successes, one failure, three contained process
jobs, the exact per-operation distribution, and one `resource_refusal`. Windows
correctly reports no Unix process-group containment. This is a real multi-job
product-shaped workflow rather than a hand-constructed metrics-only test. The
library unit test separately freezes the aggregate CSV schema and rejects empty
and incompatible-schema inputs.

## Boundary

This aggregation API reports process-client observations and configured
containment. The additive
[controller split observability contract](CONTROLLER_SPLIT_OBSERVABILITY_V1.md)
now exposes versioned internal phase durations and structural work counters for
the governed split path. Its additive allocation contract reports bounded
system-allocator events. It does not yet expose cache hit rates, allocator
peaks, CPU counters, or per-phase peak RSS. The external process-resource
benchmark remains the source for architecture-labelled peak RSS and must not be
mixed into deterministic compatibility results or routing decisions.

After fixture setup, the separate governed split acceptance runs five observed
contract invocations and retains only deterministic structural aggregates. It
is linked from the controller split observability specification.
