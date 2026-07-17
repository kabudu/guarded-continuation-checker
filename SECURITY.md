# Security policy and threat model

CQ-SAT/GCC is a research preview. Please report suspected vulnerabilities
privately through GitHub Security Advisories for this repository. Do not include
confidential RTL, logs, traces, or credentials in a public issue.

## Supported security surface

Only the latest release on `master`, Rust 1.97, Linux synthesis containment, and
artifact schema v4 receive security fixes. macOS is a development platform and
does not provide the Linux address-space containment guarantee.

## Trust boundaries

- RTL, include files, project configuration, assumptions, AIGER, and retained
  artifacts may be malformed or confidential.
- Yosys is an external parser and synthesizer running as a child process. Linux
  runs receive process-group, address-space, output-file, model-size, and
  wall-clock bounds. These limits reduce denial-of-service impact but are not a
  syscall, filesystem, or network sandbox.
- The CQ-SAT/GCC process, selected Yosys binary, operating-system account, CI
  runner, and artifact destination are trusted in the current model.
- Artifact schema v4 detects evidence changes relative to its manifest. It does
  not sign or remotely attest the manifest.

## Required deployment controls

- Run evaluations on an ephemeral, least-privilege Linux worker with no
  production credentials, no unnecessary network access, and a dedicated
  artifact directory. Treat RTL and all generated evidence as confidential.
- Pin CQ-SAT/GCC, Rust, Yosys, SymbiYosys, Z3, and CI actions to reviewed
  revisions or immutable image digests. Preserve `Cargo.lock` and build with
  `--locked`.
- Validate completed bundles before retention or consumption. Copy the manifest
  digest or a signed CI attestation into a separately trusted system when
  malicious artifact-store modification is in scope.
- Apply retention, access-control, backup, and deletion policies appropriate to
  the RTL owner. Unsafe traces can reveal internal state and environment inputs.

## Known security limitations

- Local Yosys execution is not a complete sandbox. Untrusted third-party RTL
  must be evaluated inside an additional container or VM boundary.
- Releases are not yet accompanied by signed binaries, an SBOM, SLSA provenance,
  or reproducible-build attestations.
- The project has not completed an independent external security assessment.

These limitations keep the production security gate open. They must not be
represented as certification or as safe processing of hostile RTL on a shared
workstation.
