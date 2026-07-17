# RTL artifact schema v2

Schema v2 is the first compatibility-locked CQ-SAT/GCC RTL evidence contract.
Schemas emitted before v2 were research-preview formats and are deliberately not
accepted by the strict validator.

Validate a completed bundle with:

```sh
continuation-quotient-sat firmware-artifact-validate ARTIFACT_DIR
```

Successful validation exits 0. Missing, reordered, duplicated, unknown, or
malformed manifest fields; missing evidence; stale source snapshots; and report
status/schema disagreement exit 2.

## Manifest contract

`run-manifest.txt` is UTF-8 key/value text, limited to 65,536 bytes. It is
published last and therefore acts as the bundle-completion marker. Every line is
`KEY=VALUE`; keys are unique and values are non-empty. Fields occur in this exact
order:

1. `status` (`SAFE` or `UNSAFE`)
2. `schema_version` (`2`)
3. `firmware_cli_version` (`1`)
4. `source`
5. `source_count` (1–64)
6. `source_0` through `source_N`, in source order
7. `source_revision`
8. `source_bytes`
9. `assumption_source`
10. `assumption_count` (0–256)
11. `top`
12. `horizon`
13. `synthesis_timeout_seconds`
14. `containment_platform`
15. `process_group_timeout_kill`
16. `synthesis_memory_limit_kind`
17. `synthesis_memory_limit_bytes`
18. `synthesis_file_limit_bytes`
19. `yosys`

Percent signs, line feeds, and carriage returns in source labels are encoded as
`%25`, `%0A`, and `%0D`. Consumers must not interpret other percent sequences.

One source is stored as `source.sv`; multiple sources are stored as
`source-0000.sv` through `source-NNNN.sv`. An `assumptions.txt` snapshot exists
exactly when `assumption_count` is non-zero. The validator also requires
`model.aag`, `signal.map`, `synthesis.ys`, `yosys.log`, `yosys-errors.log`,
`solver-metrics.csv`, and `safety-report.txt`.

## Safety-report contract

The first three lines of `safety-report.txt` are exactly:

```text
status=SAFE|UNSAFE
schema_version=2
firmware_cli_version=1
```

The status and versions must agree with the manifest. Remaining metadata and an
optional UNSAFE trace follow; consumers that need only bundle validity may stop
after the validated prefix.

## Compatibility policy

- The v2 validator accepts only schema v2 and rejects unknown fields.
- Reordering, removing, renaming, or adding a field requires a new schema
  version and parallel compatibility tests.
- A future producer may retain the v2 validator, but must not emit changed data
  while claiming `schema_version=2`.
- Regression tests generate SAFE and constrained bundles, validate the exact
  v2 field order, and reject status disagreement and unknown-field drift.
