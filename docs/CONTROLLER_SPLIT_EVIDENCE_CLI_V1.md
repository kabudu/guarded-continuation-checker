# Controller split-evidence CLI v1

Status: implemented self-service file interface. Not a production release.

## Purpose

The split-evidence interface stores one proof-carrying controller MTBDD
separately from plant-local result batches. It permits a consumer process to
check and admit the controller proof once, then verify several independently
replaceable plant batches without repeating that proof check.

The interface preserves the exact controller, source, wiring, property,
initial-state, horizon, and ordered-member bindings. It does not weaken or
replace exact verification.

## Capability discovery

```sh
guarded-continuation-checker controller-split-evidence-cli-version
```

The one-line response is strict and versioned. Version 1 advertises canonical
controller and plant artifact versions, manifest and artifact limits, one-time
admission, complete ordered obligation binding, and fail-closed unsupported
input handling.

## Produce controller evidence

```sh
guarded-continuation-checker \
  certify-controller-proof-evidence-v1 \
  controller-plant-manifest.txt \
  controller.controller-evidence
```

The manifest supplies the controller source, AIGER model, relevant inputs, and
observed outputs. Plant entries are loaded and snapshotted by the shared
manifest parser but are not encoded into the controller artifact. Repeating the
command with a different output path produces byte-identical evidence when the
controller boundary is unchanged. Existing outputs are never overwritten.

## Produce plant-local results

```sh
guarded-continuation-checker \
  certify-bound-plant-results-v1 \
  controller-plant-manifest.txt \
  controller.controller-evidence \
  batch.plant-results
```

The controller proof is checked before result production. The output binds its
SHA-256, complete ordered manifest obligations, and exact replayed answers.

## Verify several batches with one admission

```sh
guarded-continuation-checker \
  verify-bound-plant-result-set-v1 \
  controller.controller-evidence \
  original-manifest.txt original.plant-results \
  replacement-manifest.txt replacement.plant-results
```

The process checks the controller proof once, verifies each manifest uses the
same controller model, source digest, and MTBDD boundary, then replays every
plant batch. It emits one observable row per batch and a final aggregate with
`controller_admissions=1`. Any malformed pair, controller drift, obligation
drift, integrity failure, or semantic mismatch returns no verified aggregate.

## Security and operational boundary

- Inputs are bounded regular files; symlinks and oversized files are rejected.
- Output creation is exclusive and uses mode `0600` on Unix.
- Canonical decoders reject truncation, mutation, malformed counts, and trailing
  bytes.
- The set command accepts at most 64 manifest/result pairs.
- `ControllerSplitEvidenceTool` is the typed Rust process client. It discovers
  and enforces the versioned contract, invokes the executable without a shell,
  applies time, output, file, and supported-platform memory bounds, and reports
  stable invocation metrics.
- Typed response parsing requires canonical fields and versions, checks every
  batch index and answer count, uses checked aggregate arithmetic, reconciles
  member, answer, reachable-state, and transition totals, and requires exactly
  one controller admission.
- The CLI applies static compiled artifact limits. Explicit caller-selected
  resource policies can narrow them for each deployment.

## Governed multi-batch verification

Discover the separate resource contract:

```sh
guarded-continuation-checker controller-split-resource-cli-version
```

Run the same one-admission verification under a canonical caller policy:

```sh
guarded-continuation-checker \
  verify-bound-plant-result-set-with-resources-v1 \
  controller.controller-evidence \
  examples/controller-split-resource-policy-v1.txt \
  original-manifest.txt original.plant-results \
  replacement-manifest.txt replacement.plant-results
```

The policy independently bounds controller evidence bytes and its embedded
UNSAT proof, batch count, per-batch plant bytes, members, horizon, product
states, and conservative transition evaluations. It also bounds total plant
bytes, members, and conservative transition evaluations across the request.

GCC preflights the complete set before semantic replay. It snapshots canonical
manifest semantics, source/model digests, result digests, and resource
assessments, then checks those snapshots again during verification. Input drift,
any tighter limit, malformed policy text, arithmetic overflow, or an unknown
refusal fails closed. A policy refusal exits with code 3, emits no verified row
or logical answer, and uses one of eleven versioned refusal reasons.

Rust callers can use `ControllerSplitResourceTool` for the same path. It
strictly validates discovery and result contracts, applies bounded shell-free
execution, converts known exit-code-3 reasons into typed `ResourceRefused`
errors, and rejects inconsistent or overflowing helper summaries.

## Typed Rust client

```rust,no_run
use guarded_continuation_checker::ControllerSplitEvidenceTool;
use std::path::Path;

let tool = ControllerSplitEvidenceTool::discover(
    "guarded-continuation-checker",
)?;
let evidence = Path::new("controller.controller-evidence");
let manifest = Path::new("controller-plant-manifest.txt");
let results = Path::new("batch.plant-results");

tool.certify_controller_evidence(manifest, evidence)?;
tool.certify_plant_results(manifest, evidence, results)?;
let summary = tool.verify_set(evidence, &[(manifest, results)])?;
assert_eq!(summary.controller_admissions, 1);
# Ok::<(), guarded_continuation_checker::PredicateApiError>(())
```

The client rejects empty sets and requests above the discovered batch limit
before process creation. A non-zero process exit, timeout, output overflow,
non-canonical response, inconsistent aggregate, or changed contract produces no
typed verified summary.

## Retained acceptance and compatibility seed

Run:

```sh
scripts/run-controller-split-resource-acceptance.sh \
  target/debug/guarded-continuation-checker \
  /tmp/controller-split-resource-acceptance.csv
diff -u results/controller-split-resource-acceptance-v1.csv \
  /tmp/controller-split-resource-acceptance.csv
```

The acceptance uses two independently generated, one-property batches over the
public washing controller and physical plant. It retains two exact UNSAFE
answers, one controller admission, structural byte and transition totals, six
job outcomes, and SHA-256 fingerprints for both manifests and all three split
artifacts. Controller-budget, batch-budget, total-member, malformed-policy, and
corrupt-evidence controls must return their predeclared outcomes with no partial
verified output. Timings are deliberately omitted because they are observations,
not cross-host reproducibility claims.

These fingerprints seed compatibility history for the experimental v1
contracts. Compatibility through a later tagged release is still required.
The executable release gate and forward-support rules are defined in
[`COMPATIBILITY_AND_MIGRATION.md`](COMPATIBILITY_AND_MIGRATION.md).
Hosted Linux run 29776279270 passes the governed CLI and typed-client integration
on exact commit `1227d50`. Hosted Linux run
[29777543062](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29777543062)
reproduces the retained acceptance CSV on exact commit `e74828f` and also passes
the portable API, public RTL, dependency-audit, and reproducible-bundle jobs.

Whole-process resource observations are retained separately in
[`CONTROLLER_SPLIT_PROCESS_RESOURCES_V1.md`](CONTROLLER_SPLIT_PROCESS_RESOURCES_V1.md).
They measure controller production, both plant-result producers, and governed
verification without mixing host-dependent timings into this deterministic
compatibility seed.
