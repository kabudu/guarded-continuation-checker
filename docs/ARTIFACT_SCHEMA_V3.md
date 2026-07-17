# RTL artifact schema v3

Schema v3 extends the compatibility-locked v2 contract with immutable project
configuration, include snapshots, parameter overrides, and clock/reset policy.
The active strict validator deliberately accepts only v3.

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
2. `schema_version` (`3`)
3. `firmware_cli_version` (`2`)
4. `source`
5. `source_count` (1–64)
6. `source_0` through `source_N`, in source order
7. `source_revision`
8. `source_bytes`
9. `assumption_source`
10. `assumption_count` (0–256)
11. `project_config` (`none` or `cq-project.conf`)
12. `include_dir_count`
13. `include_file_count`
14. `include_bytes`
15. `parameter_count`
16. `parameters`
17. `clock_policy`
18. `reset_policy`
19. `top`
20. `horizon`
21. `synthesis_timeout_seconds`
22. `containment_platform`
23. `process_group_timeout_kill`
24. `synthesis_memory_limit_kind`
25. `synthesis_memory_limit_bytes`
26. `synthesis_file_limit_bytes`
27. `yosys`

Percent signs, line feeds, and carriage returns in source labels are encoded as
`%25`, `%0A`, and `%0D`. Consumers must not interpret other percent sequences.
`reset_policy` is `unspecified`, `none`, `SIGNAL:deasserted-low/high`, or the
config-v2 startup form `SIGNAL:active-low/high:N`.

One source is stored as `source.sv`; multiple sources are stored as
`source-0000.sv` through `source-NNNN.sv`. An `assumptions.txt` snapshot exists
exactly when `assumption_count` is non-zero. The validator also requires
`model.aag`, `signal.map`, `synthesis.ys`, `yosys.log`, `yosys-errors.log`,
`solver-metrics.csv`, and `safety-report.txt`.

## Safety-report contract

The first three lines of `safety-report.txt` are exactly:

```text
status=SAFE|UNSAFE
schema_version=3
firmware_cli_version=2
```

The status and versions must agree with the manifest. Remaining metadata and an
optional UNSAFE trace follow; consumers that need only bundle validity may stop
after the validated prefix.

## Compatibility policy

- The active validator accepts only schema v3 and rejects unknown fields.
- Reordering, removing, renaming, or adding a field requires a new schema
  version and parallel compatibility tests.
- A future producer may retain the v3 validator, but must not emit changed data
  while claiming `schema_version=3`.
- Regression tests generate SAFE and constrained bundles, validate the exact
  v3 field order, and reject status disagreement and unknown-field drift.
