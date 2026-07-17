# Firmware CLI contract v2

Contract v2 adds the versioned RTL project-config gate while retaining the v1
commands and exit meanings. Query the active contract without touching files:

```sh
continuation-quotient-sat firmware-cli-version
```

The single output line is:

```text
firmware_cli_version=2 artifact_schema_version=3
```

## Commands

```text
firmware-cli-version
firmware-rtl-config-safety-gate PROJECT.conf ARTIFACT_DIR
firmware-safety-gate INPUT.aag HORIZON ARTIFACT_DIR
firmware-rtl-safety-gate INPUT.sv TOP HORIZON ARTIFACT_DIR
firmware-rtl-project-safety-gate TOP HORIZON ARTIFACT_DIR SOURCE.sv [SOURCE.sv ...]
firmware-rtl-constrained-project-safety-gate TOP HORIZON ARTIFACT_DIR ASSUMPTIONS.txt SOURCE.sv [SOURCE.sv ...]
firmware-artifact-validate ARTIFACT_DIR
```

`HORIZON` is an unsigned decimal integer and is clamped to at least one. `TOP`
is a simple Verilog identifier. Project source order is significant. The
assumptions grammar and project bounds are documented in the README.

`PROJECT.conf` v1/v2 uses strict `KEY=VALUE` lines and requires `top`, `horizon`,
one or more `source`, `clock`, and `reset`. It accepts bounded `include_dir`,
`parameter=NAME:VALUE`, and `assumptions` entries. Paths are relative to the
config, may not traverse, and are snapshotted before synthesis.
Version 2 adds `reset=SIGNAL:active-low:N` and `active-high:N`; `N` must be from
1 through the horizon. The reset is asserted for frames `0..N-1` and
deasserted for all remaining frames. Version 1 remains accepted unchanged.

## Exit status

- `0`: a safety gate produced SAFE, the version query succeeded, or a bundle is
  valid.
- `1`: a safety gate produced a reproducible UNSAFE result.
- `2`: usage, input, tool, resource, synthesis, solver, publication, or artifact
  validation failure.

Diagnostics are written to stderr for exit 2. Safety gates may emit GitHub
Actions annotations and human-readable progress to stdout; only the version
query's single key/value line is intended for direct machine parsing.

## Compatibility policy

- Argument order, command names, exit meanings, and the version-query format are
  stable for contract v2.
- A breaking change requires a new CLI contract version and compatibility tests.
- Additive commands may be introduced without changing v2, but existing v2
  commands cannot silently change semantics.
- Every schema-v3 report and manifest records `firmware_cli_version=2`.
