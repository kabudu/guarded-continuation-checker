# Production-evaluation operations runbook

This runbook operates CQ-SAT/GCC as a design-partner evaluation service. It does
not turn the research preview into a certified product. The production claim
remains governed by `PRODUCTION_READINESS.md`.

## Supported environment

- Ephemeral, least-privilege Linux worker; do not use a developer workstation.
- Linux cgroup v2 and a Docker daemon reporting its built-in seccomp profile.
- Rust 1.97 and edition 2024 for source builds.
- The release tag and checksum-locked `Cargo.lock` from this repository.
- Yosys on `PATH`. Record its full `yosys -V` output for every deployment.
- No production credentials and no network access beyond what the deployment
  explicitly needs. Hostile third-party RTL requires an additional container or
  VM boundary as described in `SECURITY.md`.

macOS is development-only because it cannot enforce the Linux 2 GiB Yosys
address-space limit. Windows is unsupported.

For untrusted partner RTL, build a static Linux binary and provision the pinned
isolation image during a trusted network-enabled setup phase:

```sh
rustup target add x86_64-unknown-linux-musl --toolchain 1.97
cargo +1.97 build --release --locked --target x86_64-unknown-linux-musl
docker pull \
  hdlc/yosys@sha256:58c0c80e41fd96b4b90da53c730aa3c43051f0cf2a6c6e336bd012281479df22
```

Run the evaluation with no runtime image pull or network access:

```sh
scripts/isolated-rtl-evaluation.sh \
  target/x86_64-unknown-linux-musl/release/continuation-quotient-sat \
  cq-project.conf evidence/run-001
```

See [hostile-RTL isolation profile v1](ISOLATION_PROFILE_V1.md) for enforced
controls, exit semantics, evidence, and the cases that still require a VM.

## Install and qualify

Build from a reviewed release tag; replace `VERSION` with the intended version:

```sh
git clone https://github.com/kabudu/continuation-quotient-sat.git
cd continuation-quotient-sat
git checkout VERSION
rustup toolchain install 1.97
cargo +1.97 build --release --locked
```

Install Yosys through the organization's approved, pinned package or container
process. Then run the fail-closed qualification check in a new directory:

```sh
scripts/production-evaluation-check.sh \
  target/release/continuation-quotient-sat \
  /tmp/cq-production-evaluation-check
```

Qualification succeeds only if a known SAFE case exits 0, a known UNSAFE case
exits 1, both schema-v4 bundles validate, and Linux containment fields match the
enforced limits. Any other outcome blocks rollout. Delete the temporary evidence
after recording the release commit, Rust version, Yosys version, host image, and
check result in the deployment record.

## Run an evaluation

Use a fresh, dedicated artifact directory and a version-controlled project
config. A completed result has exit 0 (SAFE) or 1 (UNSAFE); exit 2 is a tool or
input failure and must never be treated as SAFE.

```sh
set +e
continuation-quotient-sat firmware-rtl-config-safety-gate \
  cq-project.conf evidence/run-001
status=$?
set -e
case "$status" in 0|1) ;; *) exit "$status" ;; esac
continuation-quotient-sat firmware-artifact-validate evidence/run-001
```

Retain stdout, stderr, the exact command, exit status, CQ commit/tag, host image,
and the complete validated artifact directory. A manifest is published last; a
missing or invalid manifest means the run is incomplete.

## Upgrade and rollback

1. Read every changelog entry between the deployed and candidate versions.
2. Build the candidate with its own `Cargo.lock`; never reuse a previous target
   directory as release evidence.
3. Run the qualification check and a representative non-confidential regression
   corpus in an isolated staging worker.
4. Confirm the firmware CLI and artifact schema versions. Keep the old binary
   available when retained bundles require its historical validator.
5. Roll out to one evaluation worker, inspect a SAFE and UNSAFE bundle, then
   continue gradually. Do not run mixed versions into the same artifact path.

Rollback means stopping new work, restoring the previously qualified binary and
host image, and rerunning qualification. Never rewrite existing evidence to a
different schema. Record the rollback reason and affected run identifiers.

## Monitoring and failure handling

Alert on exit 2, timeout, process termination, failed bundle validation, disk
pressure, or a mismatch between expected and reported tool versions. Track SAFE,
UNSAFE, and error counts separately; a drop in errors is not evidence of safety.

On an operational failure:

1. Stop accepting new work on the affected worker.
2. Preserve the command, exit status, stderr, host and tool versions, and any
   incomplete staging directory without exposing confidential RTL.
3. Reproduce only on an isolated worker with the same immutable inputs.
4. File a private security advisory for suspected vulnerabilities; otherwise
   open a minimal GitHub issue without proprietary artifacts.
5. Ship a reviewed fix through normal CI, qualification, and staged rollout.
6. Re-run affected evaluations from the immutable source inputs. Do not edit a
   prior result in place.

There is currently no commercial support SLA. Evaluation owners must name an
internal operator and escalation contact before onboarding a design partner.

## Evidence retention and disposal

Treat source snapshots, assumptions, synthesized models, logs, and unsafe traces
as confidential design data. Store each completed bundle read-only with access
logging and a separately trusted copy of the manifest digest or signed CI
attestation. Validate after transfer and before use.

The RTL owner—not CQ-SAT/GCC—sets the retention period. Document the purpose,
owner, access group, expiry, legal hold, backup location, and deletion method
before the first run. On expiry, delete primary and backup copies according to
that policy and retain only non-sensitive operational metadata when authorized.
Never upload a partner bundle to a public issue or repository.

## Service restoration drill

At least once before a pilot, and after a material toolchain change:

1. Provision a clean worker from the recorded host image.
2. Restore the pinned source release and Yosys package without copying a build
   directory from the old worker.
3. Rebuild with `--locked` and run the qualification check.
4. Restore a permitted test bundle, validate it, and compare its retained
   manifest digest with the trusted deployment record.
5. Record elapsed time, deviations, and corrective actions.

A drill that depends on undocumented state is a failed drill.
