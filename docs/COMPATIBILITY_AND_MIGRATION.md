# Compatibility and migration policy

Status: release-candidate policy. The support clock begins only with the first
production release. Guarded Continuation Checker v0.28.0 and the current
unreleased work remain an evaluation-ready research prototype.

## Compatibility boundary

Compatibility is defined independently for:

- Rust crate APIs;
- executable names and command arguments;
- machine-discovered CLI contracts;
- canonical policy and manifest text;
- binary evidence artifacts; and
- published result and metrics schemas.

A matching major contract or artifact version means the existing syntax and
semantics remain accepted. A breaking syntax, meaning, required-field, exit-code,
or verification change requires a new major contract or artifact version. GCC
must never silently reinterpret old evidence under new semantics.

Capability responses are strict records. Adding a field to an existing strict
response is therefore breaking and requires a new discovery version. New
commands may be added without changing an existing command.

The split allocation-observability v1 interface follows that rule: it has a
separate discovery command, verification command, typed client, and final row.
The resource and phase-observability v1 responses remain unchanged.
The split cache-observability v1 interface follows the same additive pattern
and leaves every preceding command on its original uncached execution path.

Before the first production tag, the BTOR2 parser was extended to accept
standard `output` observations and optional expression symbols.
`Btor2Model::inputs()` now reports semantic inputs reachable from transitions,
constraints, or bad properties rather than unused declared synthesis inputs.
Callers that need a raw declaration inventory must not infer it from this API.
The change is covered by external-consumer tests on every hosted platform and
will become part of the first tagged compatibility baseline.

Property-free BTOR2 component ingestion is additive. The new component parser
requires caller-selected semantic roots and does not change the retained
whole-model parser, which still requires a bad property. Revision-local public
APIs and CLI commands use the component parser with their explicit output
projections. Existing property-bearing sources remain valid and retain the same
certificate formats. The first production tag must freeze the component-root
ordering and semantic-input discovery rules.

Bounded search certificate v2 is additive. State-only bad properties continue
to produce the original byte-for-byte v1 format. V2 is selected only when the
bad property depends on the current one-bit semantic input, and records that
terminal input separately from the inputs that select transitions. The decoder
accepts both versions, while the verifier rejects cross-version
reinterpretation, missing terminal inputs, and version downgrades. The first
production tag must freeze the first two search formats and their retained
fingerprints.

Bounded search certificate v3 is additive for models with two through eight
one-bit semantic inputs. It replaces neither retained format. V3 binds the
ordered input-node list, packed transition valuations, and a distinct UNSAFE
terminal valuation. Verification rejects input reordering, noncanonical high
bits, missing values, and forced downgrade. The first production tag must also
freeze v3 and keep the existing v1 and v2 fingerprints byte-identical.

Bounded search certificate v4 is additive for models containing one or more
BTOR2 constraints. It binds the ordered constraint-node and semantic-input
lists, preserves packed transition and terminal valuations, and allows only
canonical empty suffix layers after an assumption dead end. Constraint-free
models continue to select v1, v2, or v3 under their original rules and retain
byte-identical evidence. V4 cannot be downgraded or decoded as a prior format.
The first production tag must freeze all four search formats.

Bounded search certificate v5 is additive for models with at least one
semantic input wider than one bit and at most eight total semantic input bits.
It binds the ordered input identifiers and their source widths, with packed
valuation offsets derived only from that canonical order. Constraints may be
present or absent. Boolean-only models continue to select v1 through v4 under
their original rules and encodings. The public `SearchCertificate` structure
adds `input_widths`; callers constructing values directly must initialise it to
an empty vector for v1 through v4. The first production tag must freeze all five
formats. Hosted run 29874337371 closes the v5 public-design and compatibility
gates.

The predicate-set Rust module and the `check-btor2-predicate-set` and
`verify-btor2-predicate-set` commands are additive. They do not alter bounded
portfolio v3 or any existing certificate decoder. Certificate v2 adds joint
SAFE and UNSAFE recurrence evidence over one recurrence. Certificate v3 adds
invariant-chained evidence over multiple recurrences and becomes the current
producer format.

The decoder and verifier continue to accept retained v1 and v2 artifacts under
their original routing and member semantics; they are never reinterpreted as
v3. The original ordered property list and horizon remain external verifier
inputs in every version. A future change to ordering, routing, witness meaning,
or member semantics requires a new major artifact version. The first production
tag must freeze retained v1, v2, and v3 fingerprints before this API can enter
the supported window.

Revision batch certificate v1 is an experimental research format. Its public
Rust types permit downstream evaluation, but they are outside the first
production support window until hosted byte identity, a retained compatibility
fingerprint and the production release gate explicitly admit the format. A
pre-production batch must not be used as durable evidence without retaining the
extracted standalone certificates.

## First production-line guarantee

The first production line is limited to the eight commands in
`PRODUCTION_SUPPORT_PROFILE_V1.md`. Research-build command or artifact versions
do not enter the production support window merely because their implementation
is present in the same repository. The profiled binary rejects those commands
before research dispatch.

Beginning with the first production tag:

1. released Rust APIs follow SemVer;
2. a released CLI or artifact major version remains readable and verifiable for
   at least the next two minor releases and at least 12 months, whichever is
   longer;
3. deprecation is announced in the changelog before removal and names the exact
   replacement and last supporting release;
4. old evidence remains immutable and is never upgraded in place;
5. verification of an unsupported version fails closed without a SAFE or
   UNSAFE answer; and
6. every release candidate runs all retained compatibility baselines from its
   supported window.

The project may extend a support window but must not shorten a published window
retroactively. Security fixes may disable unsafe production, but the release
must still identify the affected versions and provide a non-ambiguous failure.

## Split-evidence v1 baseline

The candidate split controller, plant-result, manifest, policy, discovery, and
refusal contracts are frozen by:

```sh
scripts/check-controller-split-compatibility-v1.sh \
  target/debug/guarded-continuation-checker
```

The gate invokes the real executable. It regenerates the retained public
two-batch acceptance, verifies both UNSAFE answers under one controller
admission, exercises three resource refusals and two invalid-input controls,
and compares the complete deterministic CSV with
`results/controller-split-resource-acceptance-v1.csv`. That CSV binds exact
manifest, controller-evidence, and plant-result SHA-256 fingerprints.

Passing this gate on the current candidate establishes a baseline, not history.
The first subsequent tagged release that passes the same gate will establish
cross-tag compatibility evidence. Until then, documentation must continue to
say that tagged compatibility history is open.

## Upgrade procedure

1. Preserve the deployed binary, lockfile, release bundle, and trusted hashes.
2. Read every intervening changelog entry and identify contract-version changes.
3. Verify the candidate release bundle before execution.
4. Run the complete retained compatibility suite and production qualification
   on an isolated staging worker.
5. Validate representative old evidence with the candidate binary without
   rewriting it.
6. Generate new staging evidence and independently check its logical results.
7. Roll out to one worker, keep artifact directories version-separated, and
   expand only after SAFE, UNSAFE, refusal, and invalid-input controls pass.

An upgrade is rejected if an old supported artifact changes meaning, a strict
contract drifts without a major version change, a refusal becomes an answer, or
the candidate cannot verify retained evidence inside its support window.

## Rollback procedure

Stop new work, restore the previously qualified binary and host image, and run
that release's qualification check. Do not ask the old binary to interpret
evidence produced only by a newer major format. Keep old and new evidence in
separate immutable directories, retain the failed upgrade diagnostics, and
rerun affected source inputs rather than editing prior results.

Rollback compatibility is directional: a new binary must support declared old
formats, while an old binary is not expected to understand a future format.
The deployment record must therefore bind each result to its producer and
validator versions.

## Rust API checks

Public integration tests under `tests/*_api.rs` compile as external consumers
of the library target. Every supported release runs them under the minimum
documented Rust toolchain. Before the first crate publication, release
automation must also compare the candidate public API with the most recent
published crate and reject unversioned SemVer breakage.

No crate has been published yet, so a registry-to-candidate SemVer comparison
cannot currently provide evidence. This remains a first-publication release
gate rather than being represented as complete.
