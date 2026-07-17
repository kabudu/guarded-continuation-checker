# Firmware CLI contract v1

Contract v1 is the first stability commitment for CQ-SAT/GCC's product-facing
firmware commands. Query the active contract without touching project files:

```sh
continuation-quotient-sat firmware-cli-version
```

The single output line is:

```text
firmware_cli_version=1 artifact_schema_version=2
```

## Commands

```text
firmware-cli-version
firmware-safety-gate INPUT.aag HORIZON ARTIFACT_DIR
firmware-rtl-safety-gate INPUT.sv TOP HORIZON ARTIFACT_DIR
firmware-rtl-project-safety-gate TOP HORIZON ARTIFACT_DIR SOURCE.sv [SOURCE.sv ...]
firmware-rtl-constrained-project-safety-gate TOP HORIZON ARTIFACT_DIR ASSUMPTIONS.txt SOURCE.sv [SOURCE.sv ...]
firmware-artifact-validate ARTIFACT_DIR
```

`HORIZON` is an unsigned decimal integer and is clamped to at least one. `TOP`
is a simple Verilog identifier. Project source order is significant. The
assumptions grammar and project bounds are documented in the README.

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
  stable for contract v1.
- A breaking change requires a new CLI contract version and compatibility tests.
- Additive commands may be introduced without changing v1, but existing v1
  commands cannot silently change semantics.
- Every schema-v2 report and manifest records `firmware_cli_version=1`.
