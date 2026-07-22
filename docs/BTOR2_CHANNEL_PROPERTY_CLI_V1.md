# BTOR2 channel property CLI v1

Status: experimental self-service interface. Not a production-supported
surface.

## Purpose

The v1 file boundary makes the canonical `GCCBCP01` property portfolio usable
without linking GCC as a Rust library or preparing a structural certificate by
hand. An operator supplies:

- one separately retained BTOR2 model;
- one canonical query manifest containing channel count, semantic roots, and
  ordered properties;
- one canonical resource policy; and
- one output path for production or one existing artifact for verification.

Production derives and authenticates the repeated-region partition, plans the
complete solver batch, refuses an over-budget batch before any property solver
starts, creates the exact proof portfolio, verifies it from source, and then
publishes with no-clobber semantics. Verification reconstructs all structural
and property semantics from the separately supplied model and manifest.

## Version discovery

```console
guarded-continuation-checker btor2-channel-property-cli-version
```

The one-line response freezes CLI, artifact, manifest, and policy versions;
all static limits; the exact static route; fail-closed fallback semantics; and
source-replay verification.

Optional phase observations have an independent additive discovery contract:

```console
guarded-continuation-checker btor2-channel-property-observability-cli-version
```

It freezes phase schema v1 and states that timings use no calibration, never
participate in correctness, and are unavailable on failure.

## Query manifest

```text
gcc-btor2-channel-properties-v1
channels=6
semantic_roots=9,39
query=0,0,output-high,2
query=1,1,output-high,2
query=2,0,output-low,2
status=complete
```

Query identifiers must be strictly increasing. Channels must be in range,
roots must be nonzero and strictly increasing, and property names are exactly
`output-high` or `output-low`. The file must be canonical UTF-8 LF text with a
final newline and no NUL, CR, duplicate, missing, reordered, or trailing field.

## Resource policy

```text
channel_property_policy_version=1
max_queries=4096
max_members=4096
max_evidence_bytes=67108864
max_artifact_bytes=69206016
max_projected_work=100000000000
status=complete
```

Every decimal is canonical and every value is bounded by the executable's
advertised static maximum. `max_projected_work` governs production only.
Verification reports `projected_work=not-applied`; it instead applies the
policy's query, member, evidence, artifact, and nested certificate limits.

## Commands

```console
guarded-continuation-checker certify-btor2-channel-properties \
  model.btor2 queries.txt policy.txt result.channel-properties

guarded-continuation-checker verify-btor2-channel-properties \
  model.btor2 queries.txt policy.txt result.channel-properties

guarded-continuation-checker certify-btor2-channel-properties-observed \
  model.btor2 queries.txt policy.txt result.channel-properties

guarded-continuation-checker verify-btor2-channel-properties-observed \
  model.btor2 queries.txt policy.txt result.channel-properties
```

Success emits one aggregate line and one result line per ordered query. Result
lines include the answer, earliest bad frame, exact backend, representative
channel, and recovered witness valuation count. Production resource refusal
uses exit code 3, a versioned reason, `result=none`, and creates no artifact.
Malformed, stale, substituted, symlinked, noncanonical, or corrupt input uses
exit code 2 and returns no verified aggregate.

The producer opens output with create-new mode and restrictive Unix
permissions. An existing path is never overwritten. All input reads are
bounded, require ordinary files, and reject symlinks where the operating system
provides `O_NOFOLLOW`.

The observed commands preserve the base aggregate and ordered result rows, then
append one strict phase-metrics row. Certification measures input loading,
structural admission, aggregate preflight, proof construction, canonical
encoding, artifact decoding, independent source replay, and publication.
Verification reports the same schema and sets production-only phases to zero.
The implementation tests that the measured phase sum does not exceed total
time and that observed and unobserved production emit byte-identical evidence.
No duration changes admission, routing, an answer, or certificate bytes.

## Executable acceptance

```console
cargo test --release --locked --test btor2_channel_property_cli
```

The acceptance test runs a twelve-property six-channel horizon-2 production
and independent verification workflow. It retains six exact bitblast members,
returns all twelve UNSAFE results and writes the frozen 1,568-byte artifact. It
also exercises exact-threshold production, one-unit resource refusal with no
output, no-clobber publication, query and source drift, artifact mutation,
CRLF and noncanonical manifests, and symlink substitution. This is simulated
self-service evidence from the repository authorship boundary, not independent
operator acceptance.

## Typed Rust process client

`Btor2ChannelPropertyTool` discovers the executable contract and invokes the
same producer and verifier without a shell. `ExecutionPolicy` applies a
deadline, stdout and stderr caps, an artifact file limit, Unix process-group
containment, and a non-macOS Unix address-space limit when configured. Every
successful aggregate and result field is parsed into
`Btor2ChannelPropertyProcessSummary`; unknown, missing, reordered,
noncanonical, or dimensionally inconsistent output fails closed.

```rust,no_run
use guarded_continuation_checker::{
    Btor2ChannelPropertyFiles, Btor2ChannelPropertyTool,
};
use std::path::Path;

let tool = Btor2ChannelPropertyTool::discover("guarded-continuation-checker")?;
let files = Btor2ChannelPropertyFiles {
    model: Path::new("model.btor2"),
    queries: Path::new("queries.txt"),
    policy: Path::new("policy.txt"),
};
let created = tool.certify(&files, Path::new("result.channel-properties"))?;
let verified = tool.verify(&files, Path::new("result.channel-properties"))?;
assert_eq!(created.results, verified.results);

let observed = tool.observability()?;
let phased = observed.verify(&files, Path::new("result.channel-properties"))?;
assert_eq!(phased.verification.results, verified.results);
# Ok::<(), guarded_continuation_checker::PredicateApiError>(())
```

The additive `Btor2ChannelPropertyObservabilityTool` strictly discovers phase
schema v1 and invokes only the observed commands. It returns the ordinary
verified summary together with typed `Btor2ChannelPropertyPhaseMetrics`.
Unknown, missing, reordered, noncanonical, overflowing, internally
inconsistent, or operation-inappropriate timing rows fail closed. Resource
refusal retains exit code 3 and still returns no phase record or logical answer.

Valid resource refusal becomes
`PredicateApiError::Btor2ChannelPropertyResourceRefused` with a typed reason
and `FailureClass::ResourceRefusal`. It never becomes a SAFE or UNSAFE result.
The executable typed-client acceptance is:

```console
cargo test --release --locked --test btor2_channel_property_tool_api
```

## Remaining gates

- Preserve the retained arm64 and hosted Linux process-resource gates.
- Reproduce the complete workflow and artifact identity on supported hosts.
- Run realistic independently sourced properties and obtain external operator
  review.

The CLI productises established exact proof and symmetry mechanisms. It does
not establish algorithmic novelty.
