# Controller split allocation observability v1

Status: stable candidate additive CLI and Rust API. Not a production release.

## Purpose

Allocation observability measures successful system-allocator operations during
one governed split request. It is a new command and response row. The existing
resource and observability v1 discovery and result records remain unchanged.

Discover the three-layer contract:

```sh
guarded-continuation-checker \
  controller-split-allocation-observability-cli-version
```

Run the allocation-observed path:

```sh
guarded-continuation-checker \
  verify-bound-plant-result-set-with-resources-allocation-observed-v1 \
  controller.controller-evidence \
  controller-split-resource-policy-v1.txt \
  original-manifest.txt original.plant-results \
  replacement-manifest.txt replacement.plant-results
```

A successful response contains the unchanged governed batch and set rows, the
unchanged phase-observability row, and one final allocation row.

## Measurement contract

Version 1 uses Rust's system allocator and measures the `policy-through-replay`
scope. It counts:

- successful allocation and zeroed-allocation calls plus requested bytes;
- deallocation calls plus layout bytes presented to the allocator; and
- successful reallocation calls plus requested replacement bytes.

These are process-wide event counts while the guard is active. They are not
live heap, retained heap, resident memory, allocator metadata, or peak RSS.
Deallocation bytes can include objects allocated before the observed scope, so
subtracting deallocated from allocated bytes is invalid. Whole-process peak RSS
remains a separately labelled external measurement.

The global allocator delegates every operation to `System` with the original
pointer and layout contract. When observation is inactive, the added normal
path is one relaxed atomic state check per allocator event. When active, an
in-flight barrier gives the final snapshot a closed concurrency boundary.
Counter overflow fails closed before any result row is written.

Five alternating release-mode controls on one Darwin arm64 development host
compared commit `7b9e024` with the instrumented working tree. Median wall time
for the complete retained acceptance was 0.74 seconds before and 0.75 seconds
after; median user time was 0.65 and 0.66 seconds. This is a local regression
control, not portable performance evidence or a routing input.

## Rust API

`ControllerSplitAllocationObservabilityTool` discovers all three strict
contracts and runs the command without a shell under the existing execution
policy. `verify_set_observed` returns
`ControllerSplitAllocationObservedSummary` and whole-process
`InvocationMetrics`. The parser rejects contract drift, missing or extra rows,
non-canonical or overflowing values, empty allocation observations, and any
invalid underlying governed or phase summary.

## Failure semantics and retained evidence

The allocation guard disables itself on every early return. Producer-side
refusal, invalid input, semantic failure, or counter overflow emits no partial
CLI row. The typed client returns no typed result for those failures, timeouts,
output failures, or hostile helper responses. Known resource refusals retain
their typed reason and process metrics.

The retained split observability acceptance now uses this allocation-observed
surface for three successful requests covering four batches, plus discovery and
one resource refusal. It requires positive allocation calls and bytes in every
successful request and no stdout on refusal. Its canonical CSV continues to
retain only portable structural counts; allocator event totals are deliberately
validated live rather than frozen across platforms and toolchains.

## Boundary

This closes the bounded allocation-event counter requirement for the governed
split path. It does not provide per-phase allocation attribution, live-heap
accounting, or an allocator-independent peak. The additive
[cache-observability contract](CONTROLLER_SPLIT_CACHE_OBSERVABILITY_V1.md)
reports integrity-preserving process-local semantic replay reuse separately.
