# Predicate certificate v2 reliability boundary

This document defines the reliability contract for checking corrupted,
truncated, oversized or otherwise invalid proof-carrying predicate
certificates. Certificate v2 remains experimental.

## Reliability objective

The verifier must preserve the integrity of the bounded
avoidable/unavoidable answer and keep ordinary malformed input from terminating
the host process. Every certificate byte, pathname and field order may be
incorrect. The operator still selects the model being checked, and the
certificate binds that model's complete SHA-256 digest.

Executable replacement, dependency replacement, operating-system failure,
cryptographic failure and resource use in the separately bounded model parser
are outside this certificate-specific contract.

## Bounded behaviour

- `symlink_metadata` accepts only regular files before reading.
- A 16-MiB serialized limit is checked before allocation; canonical UTF-8/LF,
  strict field order and bounded counts prevent ambiguous parser states.
- Each decoded native proof is limited to 1 MiB and all decoded proofs to 8 MiB.
- Dimensions are capped at 16 relevant inputs, four latches, 16 states and a
  horizon of 64; edge, witness and trace counts derive from those dimensions.
- A structural native-proof preflight bounds integers, list lengths, variables,
  literals, hash width, step count, termination and trailing data before the
  third-party checker receives the proof.
- Concrete edges and terminal witnesses are replayed against the original AIG.
- Completeness obligations are rebuilt from that AIG; relation powers,
  composition and the final answer are recomputed.
- Any parsing, proof, witness, digest or semantic failure returns an error. It
  never becomes either logical answer and produces no output artifact.
- A final unwind boundary converts an unexpected failure in
  `varisat-checker` 0.2.2 into a deterministic verification error.

## Regression evidence

`predicate_certificate_v2_corrupt_inputs_are_bounded_and_process_safe` runs
5,000 deterministic transformations of a valid canonical artifact, invalid
UTF-8, a sparse oversized file, an individually oversized proof and 128
proof-byte transformations. The release-mode test is required to complete
without termination or excessive allocation.

Not every byte change must be rejected: native proofs may contain redundant
steps whose modification leaves a valid proof. Acceptance depends on the
rebuilt proof obligation, not byte identity.

## Remaining limitations

- Official builds currently use Rust unwind semantics. The structural preflight
  is the primary guard; the unwind boundary is only a final dependency fallback.
- The solver and checker share the Varisat family and a version-specific format.
  A second independently maintained checker or standard proof format remains a
  required diversity gate.
- File/proof/count limits bound input-driven allocation, but a verifier
  wall-clock deadline and process-level memory ceiling remain part of the open
  resource-governance gate.
- Deterministic mutation regression is not exhaustive parser verification.
  Continuing automated robustness coverage remains desirable.

Rust API evaluation environments apply the documented per-call process limits.
Direct CLI deployments should provide equivalent supervision until the
resource-governance platform-evidence gate closes.
