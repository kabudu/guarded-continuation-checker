# Predicate Rust API v1

The `guarded-continuation-checker` crate exposes a versioned, typed client for
firmware build systems that need to produce and verify dense predicate
certificates without constructing shell commands.

```rust,no_run
use guarded_continuation_checker::{
    CertificateVersion, ExecutionPolicy, PredicateResult, PredicateTool,
};
use std::path::Path;
use std::time::Duration;

let policy = ExecutionPolicy::new(Duration::from_secs(30), 64 * 1024)?;
let tool = PredicateTool::discover_with_policy("guarded-continuation-checker", policy)?;
let result = tool.verify(
    CertificateVersion::V2,
    Path::new("controller.aig"),
    Path::new("controller.cert2"),
)?;
assert!(matches!(
    result,
    PredicateResult::Avoidable | PredicateResult::Unavoidable
));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Contract

`PredicateTool::discover` invokes `predicate-cli-version` directly through
`std::process::Command`; no shell interprets paths or arguments. Discovery
strictly validates CLI contract v1, certificate versions 1/2, portfolio format
v1, the native proof format, canonical field order and numeric values before a
tool handle is returned.

The public v1 types are:

- `PredicateTool`: a handle bound to one discovered executable;
- `ExecutionPolicy`: per-invocation deadline and stdout/stderr bounds;
- `PredicateCapabilities`: the executable's typed formats and limits;
- `CertificateVersion::{V1,V2}`;
- `PredicateResult::{Avoidable,Unavoidable}`; and
- `PredicateApiError`: I/O, command, compatibility and response failures.

`certify` accepts an explicit certificate version, model, bad-output index,
transcript and output path. `verify` accepts the version, model and certificate.
Both logical results are successful return values. Usage, input, resource,
proof, semantic and publication failures remain errors and are never converted
into logical answers.

The executable retains atomic/no-overwrite publication and the independent
verification semantics defined by each certificate version. The client checks
that the requested format was advertised before launching a job.

Every discovery, production and verification invocation uses the tool handle's
`ExecutionPolicy`. The default is a five-minute deadline and a 1-MiB limit for
each output stream. `ExecutionPolicy::new` accepts nonzero deadlines and output
limits from 1 byte through 64 MiB. A handle can be cloned or adjusted with
`with_execution_policy` for different pipeline stages.

On Unix, each invocation creates a separate process session. A deadline stops
the complete process group before it is reaped, so helper processes cannot keep
the pipeline waiting. Unix also applies a 32-MiB default file-size ceiling,
configurable from 1 byte through 1 GiB. Supported non-macOS Unix builds apply a
2-GiB default address-space ceiling, configurable from 64 MiB through 1 TiB.
macOS does not offer the same dependable address-space control, so
`memory_limit_bytes` is `None` there; deadlines, process groups, stream bounds
and file limits still apply. Non-Unix builds retain child deadlines and stream
bounds but report process-group and address-space containment as unavailable.

Deadline and output-bound failures have stable typed variants:
`PredicateApiError::TimedOut` and
`PredicateApiError::OutputLimitExceeded`. The latter identifies `stdout` or
`stderr` and records the configured byte limit. Command exit failures,
compatibility drift and response errors remain separately distinguishable.

## Invocation metrics schema v1

`discover_observed`, `certify_observed` and `verify_observed` return an
`Observed<T>`. A failed observed operation returns `PredicateOperationError`;
both paths contain `InvocationMetrics` with:

- schema and operation kind;
- elapsed duration;
- stdout and stderr byte counts;
- configured deadline and output limit;
- configured memory/file ceilings and whether process-group containment was
  enabled (typed fields, intentionally not added to CSV schema v1);
- process exit code when available; and
- success or a stable `FailureClass` (`policy`, `io`, `timeout`,
  `output_limit`, `exit_status`, `compatibility`, or `response`).

`InvocationMetrics::csv_header()` and `to_csv_row()` expose canonical schema v1
for build records and fleet aggregation:

```text
schema_version,operation,duration_ns,stdout_bytes,stderr_bytes,timeout_ms,output_limit_bytes,exit_code,status,failure_class
```

The schema contains no model paths or certificate contents. Callers can join it
to their own job identifier without disclosing source locations. Adding columns
requires a new metrics schema version; enum string values are stable within v1.

## Deployment boundary

API v1 is deliberately an out-of-process client. This keeps the verifier's
executable boundary, exit contract and future process-level resource controls
intact. It is not yet an in-process verifier library, and applications must ship
or provision a compatible `guarded-continuation-checker` executable.

The caller owns executable selection. Production evaluation should use an
absolute path or a deployment-controlled lookup; discovery does not download or
replace binaries.

## Compatibility

The crate follows the [predicate CLI v1 migration policy](PREDICATE_CLI_V1.md).
Breaking public Rust types or method semantics require a new major crate/API
version. Additive methods and error detail may be introduced compatibly. The
typed client rejects a future incompatible CLI contract rather than guessing.

`tests/predicate_api.rs` is compiled as a separate downstream-style crate. It
discovers the actual Cargo-built executable, produces canonical v1 and v2
artifacts from the interrupt-controller product fixture, independently verifies
both and checks their typed logical results.

Library unit tests separately prove invalid-policy rejection, process-group
deadline handling, output-limit classification and the exact metrics CSV
schema. The downstream test checks successful v1/v2 verification metrics from
the real executable.
The platform matrix exercises the non-macOS Unix address-space limit separately.
The Linux-specific regression reads the effective address-space ceiling from a
contained child and requires it not to exceed the configured 128 MiB. It passed
under the repository's Rust 1.97 toolchain in the `rust:1.97` Linux container on
18 July 2026. The ordinary `cargo test --locked` Linux CI job runs the same test
on every pull request. Reproduce the container check with:

```sh
docker run --rm -v "$PWD:/work" -w /work \
  -e CARGO_TARGET_DIR=/tmp/cq-target rust:1.97 cargo test --lib
```
