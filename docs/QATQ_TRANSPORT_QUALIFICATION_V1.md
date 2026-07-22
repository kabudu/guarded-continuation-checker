# QatQ transport qualification v1

Status: local and hosted qualification gates pass, and the upstream opaque-word
API gate is closed. Compatibility history and independent review remain open.
QatQ is not part of the supported
`firmware-rtl-v1` profile or GCC's normative evidence semantics.

## Question

Can a GCC-owned, fail-closed transport envelope use the exact QatQ 0.1.3
container to reduce storage and transfer cost for large proof-carrying revision
batches without weakening byte identity, independent semantic verification, or
resource governance?

The transport layer is deliberately separate from certificate meaning. A
receiver must recover the canonical uncompressed bytes before invoking the
existing verifier. A successful compression result cannot change a SAFE or
UNSAFE answer and cannot establish an algorithmic novelty claim.

## Frozen implementation boundary

- QatQ crate version: exactly `0.1.3`.
- QatQ source revision: release tag `v0.1.3`, commit
  `8ad5369327cd9472e356fa00bfadb161108ae0bd`.
- GCC feature: additive research-only `research-qatq-transport`.
- Opaque mapping: ordered little-endian 32-bit words through QatQ's public exact
  `u32` container API. Zero padding is permitted only in the final word and the
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

## Local result

The research-only Rust API now implements the envelope, explicit policy,
QatQ-owned pre-allocation QATC framing checks and checksum validation,
chunked recovery with SHA-256, an in-memory convenience path, and atomic
create-new file publication. Eight hostile and boundary tests cover truncated,
trailing, mutated, over-limit, non-zero padding, writer-failure, and no-clobber
behaviour. A frozen 44-byte AIGER fixture has envelope SHA-256
`9be12addcf5044e300b9c54a00b9fbf476879befa0046262994a7ab87ba8efe0`
for hosted identity comparison.

On macOS arm64, five measured trials over the independently verified OpenTitan
revision batch produced an 82,428-byte GCC envelope from 14,164,144 canonical
bytes. The envelope is 29.409347% smaller than the retained 116,769-byte zstd
level 22 long-window baseline, passing the predeclared 10% gate. Median encode
and decode times were 133.008 ms and 39.104 ms, and process peak resident memory
was 66,240,512 bytes. All three deterministic encodes matched and every decode
was bit-identical.

The qualified maintained comparison remained much smaller semantically: its 16
models and independently verified rIC3 or Certifaiger evidence contain 8,892
bytes. A 104,448-byte local archive carrying those files compressed to a
4,996-byte QatQ envelope but to 2,884 bytes with zstd. QatQ was therefore
73.231623% larger on this negative control. The evidence supports an optional
static artifact-class decision for large regular revision batches, not universal
QatQ admission and not an algorithmic novelty claim.

Retained measurements:

- `results/qatq-transport-qualification-arm64-v1.csv`;
- `results/qatq-transport-resources-arm64-v1.csv`;
- `results/qatq-transport-qualification-amd64-v1.csv`;
- `results/qatq-transport-resources-amd64-v1.csv`;
- `results/qatq-transport-hosted-amd64-v1.provenance.txt`; and
- `results/qatq-revision-batch-compression-arm64-v1.csv` for the earlier raw
  codec comparison.

Hosted run
[`29893368169`](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29893368169)
passes the full suite, dependency audit, exact crate package, public RTL corpus,
and frozen envelope identity on Linux amd64, macOS arm64, and Windows amd64.
Public RTL job `88838185189` independently regenerated and verified the
14,164,144-byte batch before reproducing the Linux result. Five Linux trials
produced the same 82,428-byte envelope and both frozen hashes, with median
encode and decode times of 456.823 ms and 79.591 ms and process peak resident
memory of 68,968,448 bytes.

All ten predeclared experiment gates pass. QatQ 0.1.3 closes the opaque-byte API
gate through upstream PRs 9 and 12 without changing the frozen GCC envelope
bytes. The remaining promotion gates are compatibility history across at least
two GCC releases and independent review.
