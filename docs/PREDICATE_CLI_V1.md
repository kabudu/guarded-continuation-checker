# Predicate CLI contract v1

This contract freezes the self-service command surface for bounded dense
predicate certificates. Query it without reading a model or creating files:

```sh
continuation-quotient-sat predicate-cli-version
```

The command emits exactly one space-delimited `key=value` line:

```text
predicate_cli_version=1 certificate_versions=1,2 portfolio_certificate_version=1 proof_format=varisat-native-0.2.2 min_relevant_inputs=9 max_relevant_inputs=16 max_latches=4 max_horizon=64 max_certificate_v2_bytes=16777216 max_proof_bytes=1048576 max_total_proof_bytes=8388608
```

Fields are additive within contract v1. Existing keys, meanings, order and
values that describe a versioned artifact cannot silently change. Consumers
must reject an unsupported `predicate_cli_version`, certificate version or
proof format before starting a verification job.

## Stable commands

```text
predicate-cli-version
certify-aiger-predicate INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt CERTIFICATE.cert
verify-aiger-predicate-certificate INPUT.aag|INPUT.aig CERTIFICATE.cert
certify-aiger-predicate-v2 INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt CERTIFICATE.cert2
verify-aiger-predicate-certificate-v2 INPUT.aag|INPUT.aig CERTIFICATE.cert2
export-aiger-predicate-v2-obligations INPUT.aag|INPUT.aig CERTIFICATE.cert2 OUTPUT_DIR
verify-aiger-counterfactual INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt EXPECTED_QUERIES REPORT.txt CERTIFICATE.cert
verify-aiger-counterfactual-report INPUT.aag|INPUT.aig TRANSCRIPT.txt REPORT.txt CERTIFICATE.cert
```

Argument order and query semantics are stable for contract v1. Output paths are
no-overwrite and are published only after successful construction. Verifiers
accept no transcript argument because the canonical certificate binds the
complete query evidence needed by its versioned semantics.

The additive export command publishes obligation bundle v1: canonical
individual DIMACS completeness obligations, an exact selector-guarded aggregate
and a source/certificate/digest manifest. It does not verify an external proof;
the pinned, resource-governed external harness is documented in
[`EXTERNAL_PREDICATE_PROOF_BASELINE.md`](EXTERNAL_PREDICATE_PROOF_BASELINE.md).

Certificate v1 remains the counterfactual portfolio format. Certificate v2 is
available for explicit production and checking, but its presence in the version
line is not a promise that the portfolio selects it.

## Exit status

- `0`: the requested artifact was created or verified, or the version query
  succeeded;
- `2`: usage, input, resource, proof, semantic, compatibility or publication
  failure.

These commands do not use exit status to encode `avoidable` versus
`unavoidable`; both are valid verified certificate results. Diagnostics go to
stderr. Human-readable success lines are informational except for the exact
single-line `predicate-cli-version` response.

## Compatibility and migration policy

- A breaking command, argument, exit-status or version-line change requires
  `predicate_cli_version=2` and parallel compatibility tests.
- A new certificate encoding requires a new certificate version; existing v1
  and v2 readers remain available during the migration window.
- A portfolio-format change requires a new portfolio report contract and cannot
  reinterpret an existing report or certificate.
- Additive commands may be introduced under CLI v1 when they do not alter an
  existing command.
- A supported certificate or CLI version receives at least one tagged minor
  release of deprecation notice before removal, followed by one tagged minor
  release in which its verifier remains available. Removal can occur only in a
  subsequent major release.
- Reliability or correctness defects may cause a producer to be disabled
  immediately, but the verifier remains available wherever safe so retained
  evidence can still be checked or migrated.

The exact contract line and strict arity are regression-tested by
`predicate_cli_v1_contract_is_machine_readable_and_strict`. Both certificate
versions also retain semantic compatibility tests over the product cohort.
