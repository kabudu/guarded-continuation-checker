# Event-contract CLI and Rust API v1

This contract freezes the self-service process boundary for bounded named-CNF
event contracts, certificate v3, and portfolio report v1. It is separate from
predicate CLI v1, whose commands and discovery line are unchanged.

Discover capabilities without reading a model or writing an artifact:

```sh
guarded-continuation-checker event-contract-cli-version
```

The command emits exactly one canonical line:

```text
event_contract_cli_version=1 certificate_version=3 portfolio_version=1 semantics=bounded-named-cnf-terminal-bad-avoidance proof_format=varisat-native-0.2.2 min_relevant_inputs=9 max_relevant_inputs=16 max_latches=4 max_horizon=64 max_contract_bytes=1048576 max_certificate_bytes=33554432 max_proof_bytes=1048576 max_total_proof_bytes=8388608
```

## Stable commands

```text
event-contract-cli-version
certify-aiger-event-contract-v3 INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt CERTIFICATE.cert3
verify-aiger-event-contract-certificate-v3 INPUT.aag|INPUT.aig CONTRACT.txt CERTIFICATE.cert3
export-aiger-event-contract-v3-obligations INPUT.aag|INPUT.aig CONTRACT.txt CERTIFICATE.cert3 OUTPUT_DIR
verify-aiger-event-contract-portfolio INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt REPORT.txt CERTIFICATE.cert3
verify-aiger-event-contract-portfolio-report INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt REPORT.txt CERTIFICATE.cert3
```

Both logical answers return exit status zero after successful verification.
Usage, parsing, compatibility, proof, resource, verification, or publication
failure returns exit status two. Diagnostics use stderr. Output paths are
no-overwrite.

## Portfolio rule

The timing-free v1 admission rule selects certificate v3 exactly when:

- relevant named input support is 9 to 16 bits;
- latch state is 1 to 4 bits;
- horizon is 1 to 64; and
- every initial latch value is declared.

No trial solve, timing measurement, learned score, or per-formula calibration
influences admission. A structural rejection uses the exact persistent CDCL
encoding of the original model and contract. A recognized BDD, cache, proof,
or certificate-size exhaustion also uses exact CDCL and records
`event-contract-resource-fallback`. Any other producer or verifier failure is
returned as an error and cannot be converted into an answer.

Portfolio report v1 binds the source and contract digests, bad output, horizon,
dimensions, gate decision, backend, reason, answer, certificate status, and
operation timings. The report verifier recomputes the gate and either checks the
v3 certificate or independently resolves the original CDCL query.

## Typed Rust API

`EventContractTool` discovers and validates the capability line before exposing
shell-free methods:

- `certify_v3` and `verify_v3`;
- `verify_portfolio`; and
- `verify_portfolio_report`.

Observed variants return invocation metrics under schema v1. `ExecutionPolicy`
applies the existing deadline, output, proof-file, Unix process-group, and
supported-Linux address-space controls independently to every process.

## Compatibility and migration

- Existing ordered keys and meanings in discovery v1 cannot change silently.
- A breaking command, argument, result, or capability change requires CLI v2.
- Certificate semantics require a new certificate version rather than
  reinterpretation of v3 bytes.
- Portfolio semantics require a new report version rather than reinterpretation
  of v1 reports.
- Additive commands may be introduced without altering existing commands.
- Deprecation requires one tagged minor release of notice and one further tagged
  minor release retaining the verifier. Removal can occur only in a later major
  release.
- A defective producer can be disabled immediately, but a safe retained-evidence
  verifier remains available through the migration window.

This compatibility promise starts with the first tagged release containing the
contract. Until that tag exists, the implementation is release-candidate
evidence, not a completed compatibility history.
