# QatQ revision-batch compression probe v1

Status: positive local storage experiment. Not integrated into the certificate
format or the first production support profile.

## Question

Can QatQ's exact typed transforms exploit regularity inside GCC's shared local
relations better than conventional lossless compression while restoring the
canonical revision batch byte-for-byte?

The input is the 14,164,144-byte OpenTitan revision batch containing three
shared relations and 16 distinct-property answers. The probe uses QatQ v0.1.1
at commit `87be0cc327a1e6a2ac94c13e584d7f4eae821c5d`. Because QatQ currently exposes
f32, f16 and bf16 exact tensor inputs rather than opaque integer words, the
batch is treated as an ordered f32 bit stream. Its length is divisible by four.
This is an experimental mapping, not a claim that proof evidence is a tensor.

## Result

Every QatQ, zstd and LZ4 output decoded to the exact original batch. The best
QatQ row uses one 3,541,036-value f32 chunk:

- raw canonical batch: 14,164,144 bytes;
- LZ4: 2,909,792 bytes;
- zstd level 3: 515,606 bytes;
- zstd level 22 with a 27-bit long-distance window: 116,769 bytes; and
- QatQ exact: 76,385 bytes.

QatQ removes 99.4607% of the raw batch, is 85.1854% smaller than zstd level 3,
and is 34.5845% smaller than the strongest measured zstd configuration. This
is a real positive result for storage and transport.

It does not establish a proof-system advantage. The qualified maintained
AIGER, rIC3 and Certifaiger model-plus-evidence package is still only 8,892
bytes, about 8.59 times smaller than the compressed GCC batch. Compression also
does not reduce semantic verification work after decode.

## Product decision

Do not add QatQ to the first production release. That would expand the
dependency, format, resource and compatibility boundary while the supported
firmware profile is being frozen. A later additive transport envelope is worth
qualifying after QatQ provides, or GCC locally defines, an exact opaque-u32
interface that does not label arbitrary proof bytes as floating-point tensor
data.

That envelope must:

1. bind the compression algorithm and parameters outside the compressed bytes;
2. impose encoded, decoded, chunk-count, chunk-size and expansion-ratio limits
   before allocation;
3. decode to a temporary bounded sink while hashing the canonical batch;
4. require the decoded SHA-256 and length before semantic verification;
5. reject trailing, reordered, corrupt and decompression-bomb inputs; and
6. retain uncompressed revision batch v1 as the normative evidence semantics.

The retained measurements are in
`results/qatq-revision-batch-compression-arm64-v1.csv`. Encoding and decoding
latency, peak memory, cross-platform identity and hostile-container behaviour
remain unmeasured, so the experiment is not yet an integration decision.
