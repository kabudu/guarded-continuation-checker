# Controller-plant batch artifact v1

Status: experimental, deterministic, bounded, and available through the public
Rust library. It is not a stable release interface.

The `GCCCPA01` binary artifact contains one canonical controller transducer and
1 to 64 ordered plant-member claims. Every member binds:

- a caller-supplied 32-byte plant source digest;
- the four ordered controller/plant wiring vectors;
- initial controller and plant states;
- the bad-output index and bounded horizon; and
- the complete SAFE or UNSAFE result, including an unsafe trace when present.

All integers have fixed little-endian encodings. Vectors and members preserve
their declared order. The artifact ends with SHA-256 over every preceding byte,
and the total encoded size is limited to 16 MiB. Decoding rejects unknown
versions, invalid tags, oversized counts, truncation, trailing bytes, integrity
failure, and embedded transducer failure.

`verify_controller_plant_artifact` receives the controller and ordered plants
separately. It checks every supplied source digest, independently decodes and
verifies the controller proofs once, recomputes every exact bounded product,
and compares each complete result with the claim. The checksum detects storage
mutation; it does not replace source binding, proof checking, or semantic
recomputation.

The downstream API test exhaustively truncates the retained artifact at every
byte and flips one bit at every byte position. Every altered artifact must fail.
This is complete mutation coverage for that fixture, not a general cryptographic
security proof.

## MTBDD batch variant

The experimental `GCCMPA01` variant replaces the embedded cube transducer with
one canonical controller MTBDD. It retains the same ordered source-bound member
records, result encoding, 64-member limit, 16 MiB byte limit, and whole-artifact
integrity trailer. Its independent verifier checks MTBDD source equivalence
once, then recomputes and compares every complete member result.

The [controller MTBDD plant CLI v1](CONTROLLER_MTBDD_CLI_V1.md) exposes this
variant as a canonical file workflow. The CLI additionally requires the MTBDD
boundary and every ordered member digest, wiring vector, initial state,
property and horizon to equal the supplied manifest. A valid artifact for a
different query therefore fails closed.
