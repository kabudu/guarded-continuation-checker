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
- The CLI applies static compiled limits. Explicit caller-selected resource
  policy files and the typed bounded-process wrapper remain follow-up gates.
