# BTOR2 channel-pair trace compatibility contract v1

Status: frozen for the version-1 research surface.

## Stable identities

The following identifiers are fixed:

- artifact magic: `GCCPTR01`;
- artifact version: 1;
- CLI version: 1;
- query-manifest header: `gcc-btor2-channel-pair-traces-v1`;
- resource-policy schema: `channel_trace_policy_version=1`; and
- fixed OpenTitan artifact SHA-256:
  `aac88db71d0b536a7b2c4aed4c3f37315e29a15affbdb1f260db94136e994e51`.

The public Rust artifact, query, relation, solver, plan, result, metrics and
byte-level production and verification types are additive API surface. The
repository's public API compatibility gate compares them with the latest
published crate.

## Reader and writer rule

A version-1 reader accepts only canonical version-1 bytes. Unknown artifact
versions, relations, backends, solvers, reordered members, noncanonical nested
evidence, changed sources, changed queries, trailing bytes and invalid
checksums fail closed without a logical answer.

A version-1 writer always emits the same bytes for the same canonical source,
structural admission, ordered queries, policy and exact member evidence.
Platform-dependent fields, timestamps, timing measurements and random values
are prohibited from the artifact.

## Evolution rule

Compatible implementation fixes may retain version 1 only when the frozen
bytes and semantics remain unchanged. An incompatible field, semantic,
canonicalisation or verification change requires:

1. a new magic or artifact version;
2. a separate decoder and verifier;
3. explicit capability discovery;
4. retained old-version verification fixtures;
5. a documented migration tool or a documented refusal-only boundary; and
6. cross-platform identity and downgrade tests.

No decoder may guess a later format or reinterpret an unknown enum tag.
Automatic lossy migration is prohibited.

## Current evidence

The CLI integration test freezes the 912-byte artifact and complete digest. The
codec test proves deterministic re-encoding, exhaustive truncation and
single-byte mutation rejection, and separately recomputes valid checksums after
injecting an unsupported version, relation and solver. These cases reach the
semantic decoder rather than being rejected only by the outer checksum.

Ubuntu, macOS and Windows run the CLI and codec compatibility tests in the
protected portability matrix. Tagged release-to-release history remains open
until a later artifact or crate release exists.
