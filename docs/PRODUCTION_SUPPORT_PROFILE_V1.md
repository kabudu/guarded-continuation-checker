# Production support profile v1

Status: frozen candidate boundary. Production qualification remains open until
every applicable release gate passes.

## Supported purpose

The first production line is a bounded firmware and RTL safety checker. It
accepts AIGER directly or synthesises a bounded Verilog/SystemVerilog project
through the documented Yosys isolation boundary. It returns SAFE, UNSAFE with a
replayable trace, or an operational failure. It produces and validates RTL
artifact schema v4 bundles.

The production binary is built with Cargo feature `production-firmware`. It
must identify itself with exactly:

```text
production_support_profile=firmware-rtl-v1 firmware_cli_version=2 artifact_schema_version=4
```

The profile supports only:

```text
production-profile-version
firmware-cli-version
firmware-rtl-config-safety-gate
firmware-safety-gate
firmware-rtl-safety-gate
firmware-rtl-project-safety-gate
firmware-rtl-constrained-project-safety-gate
firmware-artifact-validate
```

Argument order and exit meanings are inherited from firmware CLI contract v2.
Evidence semantics and limits are inherited from RTL artifact schema v4 and
the Linux hostile-RTL isolation profile.

## Explicit exclusions

All predicate, event-contract, BTOR2, revision-local, controller, MTBDD,
counterfactual, causal-analysis and experiment commands are outside v1. The
profiled binary rejects them with exit 2 before research dispatch. Their source
and normal research binary remain available for later additive releases, but
they cannot silently enter the v1 support or compatibility promise.

QatQ transport, revision batch certificate v1 and every newer research format
are also excluded. Durable v1 evidence is the uncompressed schema-v4 firmware
artifact bundle.

## Release gates

A production tag requires all of the following against the exact candidate
commit and packaged bytes:

1. the executable support-boundary check passes;
2. firmware CLI v2 and artifact schema v4 compatibility fixtures pass;
3. SAFE and UNSAFE public-product cases agree with independent maintained
   oracles and replay successfully;
4. parser, artifact and hostile-project mutation suites pass;
5. Linux process, memory, file, timeout, network and filesystem containment
   probes pass;
6. two isolated builds produce identical archives, SBOM and provenance;
7. dependency audit and repository security review have no unresolved release
   blocker;
8. operations, upgrade, rollback and evidence-retention drills pass; and
9. the independent security, technical-review and design-partner gates in
   `EXTERNAL_EVIDENCE_PROTOCOL.md` are satisfied.

Items 1 through 8 are release-candidate engineering gates. Item 9 remains the
independent condition for changing the claim from evaluation-ready release
candidate to production-grade release.

Run the local support-boundary check with:

```console
cargo build --release --locked --features production-firmware
scripts/check-production-support-profile-v1.sh \
  target/release/guarded-continuation-checker
```
