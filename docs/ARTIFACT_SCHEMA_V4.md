# RTL artifact schema v4

Schema v4 extends v3 with a SHA-256 inventory that binds every generated source
snapshot, include snapshot, synthesis input, synthesized model, log, solver
metric, and safety report. The active strict validator accepts only v4.

Validate a completed bundle with:

```sh
continuation-quotient-sat firmware-artifact-validate ARTIFACT_DIR
```

Successful validation exits 0. Missing, reordered, duplicated, unknown, or
malformed manifest fields; missing evidence; a digest mismatch; a symlinked
bundle or indexed file; stale source snapshots; and report disagreement exit 2.

## Completion and integrity contract

`run-manifest.txt` remains the last published file and the bundle-completion
marker. It is UTF-8 key/value text, limited to 65,536 bytes. The v3 fields retain
their order and meaning, followed by:

1. `evidence_digest_algorithm=sha256`
2. `evidence_index=evidence.sha256`
3. `evidence_index_sha256=HEX`, the lowercase SHA-256 of the index

`evidence.sha256` is no larger than 1 MiB and contains at most 4,096 unique,
lexicographically sorted entries in this exact form:

```text
SHA256_HEX  bundle/relative/path
```

The index excludes itself and `run-manifest.txt`; it covers every other file
created in the isolated staging directory. Paths must be UTF-8, relative,
normal-component paths without traversal or line breaks. The validator rejects
symlinks and non-regular indexed files before hashing them.

The manifest and index must be retained together. SHA-256 detects corruption or
substitution relative to a separately trusted manifest digest; it is not a
signature and does not authenticate a bundle against an attacker who can
replace the manifest and every file. A deployment requiring provenance against
that attacker must retain the manifest in an independently trusted store or add
an external signed attestation.

## Existing evidence contract

One source is stored as `source.sv`; multiple sources are stored as
`source-0000.sv` through `source-NNNN.sv`. An `assumptions.txt` snapshot exists
exactly when `assumption_count` is non-zero. The validator requires
`model.aag`, `signal.map`, `synthesis.ys`, `yosys.log`, `yosys-errors.log`,
`solver-metrics.csv`, `safety-report.txt`, `evidence.sha256`, and the declared
source, configuration, assumption, and include snapshots.

The first three lines of `safety-report.txt` remain:

```text
status=SAFE|UNSAFE
schema_version=4
firmware_cli_version=2
```

The status and versions must agree with the manifest.

## Compatibility policy

- The active producer and validator accept only schema v4.
- Schema v3 remains documented for interpreting historical v0.12.0–v0.14.0
  bundles, but the current validator deliberately rejects it.
- Reordering, removing, renaming, or adding a manifest field requires a new
  schema version and compatibility tests.
- Firmware CLI contract v2 is unchanged: command signatures and exit meanings
  remain stable, while its version query now reports artifact schema 4.
