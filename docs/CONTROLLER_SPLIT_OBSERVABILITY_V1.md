# Controller split observability v1

Status: stable candidate additive CLI and Rust API. Not a production release.

## Purpose

The observed governed split interface explains how one bounded verification
request used the verifier without weakening its answer contract. It preserves
the existing resource capability and result rows byte for byte, then adds a
strict final row after, and only after, the complete request succeeds.

Discover both the unchanged governed resource contract and the additive
observability contract:

```sh
guarded-continuation-checker controller-split-observability-cli-version
```

Run observed verification with the same evidence, policy, and manifest/result
pairs accepted by the governed v1 command:

```sh
guarded-continuation-checker \
  verify-bound-plant-result-set-with-resources-observed-v1 \
  controller.controller-evidence \
  controller-split-resource-policy-v1.txt \
  original-manifest.txt original.plant-results \
  replacement-manifest.txt replacement.plant-results
```

## Phase contract

Phase metrics version 1 reports monotonic elapsed microseconds for four
non-overlapping regions:

1. `policy-and-input`: policy loading, request validation, and evidence input;
2. `controller-admission`: controller envelope and proof admission;
3. `complete-set-preflight`: bounded loading, resource assessment, and immutable
   snapshot preparation for every batch; and
4. `semantic-replay`: exact verification of every prepared batch.

Their checked sum cannot exceed `total_micros`. The same captured total is used
by the governed resource summary, so the typed client rejects disagreement
between the base and observed rows. Timings are observations only. They are not
calibration data and never affect routing, refusal, or logical answers.

## Counter contract

The final row reports checked structural counters for controller admissions,
manifest loads, plant-artifact reads, resource assessments, batch
verifications, buffered result rows, prepared batches and members, controller
evidence bytes, plant-artifact bytes, and the conservative transition bound.
The typed client reconciles every applicable counter with the base verification
summary. Unknown fields, changed ordering, non-canonical integers, overflow,
or mismatched totals fail closed.

`ControllerSplitObservabilityTool` provides shell-free Rust discovery and
verification. `verify_set_observed` returns `ControllerSplitObservedSummary`
plus whole-process `InvocationMetrics`. Its request is bounded before process
creation by the discovered batch limit.

## Failure semantics

No phase row, verified row, partial counter, or logical answer is written when
policy admission, preflight, replay, or response validation fails. A known
resource refusal remains a typed refusal with process metrics. Other failures
remain typed execution, contract, timeout, output, or response errors. This
prevents incomplete measurements from being mistaken for successful evidence.

## Retained multi-job acceptance

Run:

```sh
scripts/run-controller-split-observability-acceptance.sh \
  target/debug/guarded-continuation-checker \
  /tmp/controller-split-observability-acceptance.csv
diff -u results/controller-split-observability-acceptance-v1.csv \
  /tmp/controller-split-observability-acceptance.csv
```

After three fixture-production processes, the workflow performs five observed
contract invocations: discovery, one two-batch verification, two single-batch
verifications, and a controller-budget refusal. Its retained CSV
aggregates three controller admissions, four verified batches, eleven manifest
loads, eight plant reads and resource assessments, seven buffered rows, exact
evidence bytes, and exact conservative transition bounds. It also proves that
the refused job contributes no partial metrics. Phase timings are validated in
each successful observed process but omitted from the retained CSV because they
are not portable evidence.

Hosted run
[29781337392](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29781337392)
passes the CLI and typed observability contract, hostile-response controls,
portable APIs on Linux, macOS, and Windows, the public RTL corpus, dependency
audit, and reproducible Linux packaging on exact commit `6393ccf`.

## Boundary

This contract exposes deterministic work counts and elapsed phase observations.
It does not report allocator peak usage, cache hit rates, CPU counters, or
per-phase peak RSS. Architecture-labelled whole-process peak RSS remains in the
separate process-resource benchmark. Neither source is valid as a per-formula
calibration or an answer-selection signal.
