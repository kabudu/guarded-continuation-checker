# QatQ transport qualification v1

Status: predeclared research experiment. QatQ is not part of the supported
`firmware-rtl-v1` profile or GCC's normative evidence semantics.

## Question

Can a GCC-owned, fail-closed transport envelope use the exact QatQ 0.1.1
container to reduce storage and transfer cost for large proof-carrying revision
batches without weakening byte identity, independent semantic verification, or
resource governance?

The transport layer is deliberately separate from certificate meaning. A
receiver must recover the canonical uncompressed bytes before invoking the
existing verifier. A successful compression result cannot change a SAFE or
UNSAFE answer and cannot establish an algorithmic novelty claim.

## Frozen implementation boundary

- QatQ crate version: exactly `0.1.1`.
- QatQ source revision used for comparison: commit
  `87be0cc327a1e6a2ac94c13e584d7f4eae821c5d`.
- GCC feature: additive research-only `research-qatq-transport`.
- Opaque mapping: ordered little-endian 32-bit words through QatQ's exact f32
  bit representation. Zero padding is permitted only in the final word and the
  canonical byte length is authenticated by the GCC envelope.
- Normative format: the existing uncompressed certificate bytes. The envelope
  is never accepted by an existing semantic certificate decoder.

The GCC envelope must bind a version, codec identifier, canonical byte length,
encoded payload length, decoded SHA-256 digest, maximum values per QatQ chunk,
and the exact QatQ payload. Its parser must reject non-canonical integers,
unknown versions or codecs, trailing bytes, inconsistent lengths, non-zero
padding, hash mismatch, and every configured resource-limit violation.

## Predeclared resource policy

The public decoder must accept an explicit policy and check it before allocating
or decoding:

- maximum envelope bytes;
- maximum canonical decoded bytes;
- maximum QatQ chunks;
- maximum encoded chunk bytes;
- maximum values per chunk; and
- maximum decoded-to-encoded expansion ratio.

All arithmetic must be checked. The decoded bytes must be produced a chunk at a
time through QatQ's prevalidated limited visitor while GCC computes the canonical
SHA-256 digest. A convenience in-memory decoder may collect that stream only
after the same limits have passed. Semantic verification remains a separate
explicit operation over the recovered canonical bytes.

## Acceptance matrix

The experiment passes only if all of the following hold:

1. Exactness: empty, non-word-aligned, random, highly repetitive, canonical
   revision-batch, and maintained-proof-package payloads recover byte-for-byte.
2. Determinism: three encodes of every retained fixture are byte-identical.
3. Integrity: mutations of the envelope header, QatQ header, body, checksum,
   digest, padding, lengths, order, and trailing data are rejected.
4. Governance: over-limit encoded size, decoded size, chunk count, chunk size,
   values per chunk, and expansion ratio are rejected before decoded output is
   committed.
5. Compatibility: default GCC and `production-firmware` builds do not include
   QatQ; all existing tests and package boundaries remain valid.
6. Portability: the same fixture produces identical envelope bytes and decoded
   SHA-256 on Linux amd64, macOS arm64, and Windows amd64 in hosted CI.
7. Operations: at least five measured encode and decode trials report median,
   minimum, maximum, encoded bytes, and process peak resident memory. No timing
   value participates in admission or correctness.
8. Compression: on the canonical 14,164,144-byte revision batch, the envelope
   must remain at least 10 percent smaller than the retained zstd level 22
   long-window result of 116,769 bytes. Every negative corpus row is retained.
9. Semantic preservation: the recovered revision batch passes the existing
   independent batch verifier and extracts the same standalone certificates.
10. Security: malformed inputs never panic, never silently fall back to raw
    bytes, and never overwrite an existing output on failure.

## Decision rule

Passing these gates qualifies an optional transport experiment, not the
production support profile. Promotion requires a stable opaque-byte API in QatQ
or removal of the floating-point-labelled mapping, compatibility history across
at least two GCC releases, dependency audit evidence, and independent review.
Failure of any gate keeps QatQ outside GCC's supported product boundary and the
negative result remains published.
