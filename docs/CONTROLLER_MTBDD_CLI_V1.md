# Controller MTBDD plant CLI v1

Status: experimental self-service interface. The contract is versioned and
fail-closed, but has no tagged compatibility history and is not production
supported.

## Discovery

```sh
guarded-continuation-checker controller-mtbdd-cli-version
```

The single canonical line reports CLI, MTBDD, plant-artifact, and manifest
versions plus every static dimension. Unknown or missing fields must be treated
as incompatibility by clients.

## Canonical manifest

Manifest v1 is canonical LF-terminated UTF-8 text. It binds controller and
ordered plant source plus ASCII AIGER paths, relevant inputs, observed actions,
all wiring vectors, initial states, bad-output indices, horizons, member order,
completion marker, and format version.

All paths are normalized relative paths containing only ordinary components.
Absolute paths, traversal, symlinks, empty vectors, unsorted or duplicate
indices, noncanonical integers, CRLF, NUL, trailing fields, oversized files,
and unsupported dimensions fail closed. The manifest is capped at 64 KiB. The
retained public example is
`corpus/rtl/wmcontroller/physical-plant-batch-v1.txt`.

## Produce and verify

```sh
guarded-continuation-checker certify-controller-mtbdd-plant-batch \
  corpus/rtl/wmcontroller/physical-plant-batch-v1.txt \
  physical-plant.mtbdd-plant

guarded-continuation-checker verify-controller-mtbdd-plant-batch \
  corpus/rtl/wmcontroller/physical-plant-batch-v1.txt \
  physical-plant.mtbdd-plant
```

Production uses create-new semantics and never overwrites an output. Both
commands verify the complete controller relation and every claimed result
before success. Verification also requires the decoded boundary, wiring,
digests, initial states, properties, horizons, and member order to equal the
manifest. Each member reports its answer, bad frame, trace length, reachable
states, and explored transitions.

## Rust API

Rust integrations can use `ControllerMtbddTool` instead of parsing subprocess
output or invoking a shell. Discovery validates the complete capability line.
`certify_observed` and `verify_observed` return a typed batch summary, ordered
typed member results, and invocation metrics. The client rejects noncanonical
numbers, changed versions, malformed member ordering, inconsistent SAFE and
UNSAFE counts, and inconsistent aggregate state or transition totals.

The API retains the process boundary and applies the same configurable timeout,
output, file, process-group, and supported memory limits as `PredicateTool`.
`tests/controller_mtbdd_tool_api.rs` is the minimal end-to-end integration
example.

## Acceptance

```sh
scripts/run-controller-mtbdd-self-service-acceptance.sh \
  target/release/guarded-continuation-checker \
  target/controller-mtbdd-acceptance.csv
```

The harness checks discovery, production, fresh-process verification, four
exact UNSAFE bad frames, two SAFE answers, manifest-drift rejection,
artifact-mutation rejection, and no-clobber behavior. The retained rows are
`results/controller-mtbdd-self-service-acceptance-v1.csv`. This is simulated
external-style acceptance, not independent partner evidence.

## Trust boundary

The AIGER files are the authoritative executable models. Source-file digests
bind provenance to the artifact, but v1 does not prove that synthesis produced
the supplied AIGER from the adjacent Verilog. Evaluators must independently
reproduce synthesis and its checked digest when that relationship matters. The
public washing-controller oracle scripts demonstrate that stronger workflow.
Source-to-model attestation remains a production-readiness gap.

Component symlinks are rejected when paths are resolved and final files are
opened with no-follow semantics on Unix. This does not turn the native CLI into
a sandbox against concurrent filesystem replacement. Evaluate attacker-owned
inputs inside the documented isolation profile.
