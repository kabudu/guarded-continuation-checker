# OpenTitan PWM symbolic firmware-class boundary v1

## Question

Can repeated OpenTitan PWM channels retain live firmware register inputs while
exposing exact, calibration-free classes suitable for later proof reuse?

## Frozen contract

The source boundary retains the exact OpenTitan PWM child sources at commit
`86db2898288664d8d5e8fc635b48951ef63e3439`. A GCC-authored harness supplies
three two-bit symbolic buses: enable, inversion, and write. Even channels use
class 0 and odd channels use class 1. Remaining register values are fixed by
class. The contract is explicit and caller-visible; GCC does not infer that
independent firmware writes are equal.

The build authenticates the upstream sources and pinned Yosys revision
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`. The default authentic models were
regenerated to temporary paths and remained byte-identical after the builder
was generalised to accept a checked harness and top module.

## Retained result

| Channels | Live input bits | States | Exact structural classes | Reused channels |
|---:|---:|---:|---|---:|
| 2 | 6 | 17 | `[0] [1]` | 0 |
| 4 | 6 | 25 | `[0,2] [1] [3]` | 1 |
| 6 | 6 | 33 | `[0,2,4] [1] [3,5]` | 3 |

The six-channel model therefore requires three representatives instead of six
for the currently extracted structural relation. Channel 1 remains a singleton
in the synthesised graph, so this experiment does not force the expected
even/odd partition or hide a failed match. Repeated derivation is deterministic.

The result checker freezes every model digest, size, state count, class, and
input count. The API test also rejects accidental constraints or embedded bad
properties. These are source-bound property-free models, not safety proofs.

## Canonical admission artifact

The follow-up artifact stores the exact model digest, semantic roots, channel
count, ordered structural signatures, and complete partition. Its decoder
preflights byte and count limits, verifies a SHA-256 envelope checksum, and
requires byte-identical canonical re-encoding. Verification takes the model as
a separate input, authenticates it, re-extracts all regions, and recomputes
every signature and class before returning an opaque admission capability.

| Channels | Artifact bytes | Artifact/model ratio |
|---:|---:|---:|
| 2 | 232 | 1.356646% |
| 4 | 348 | 1.330479% |
| 6 | 460 | 1.306521% |

Every truncation and every single-byte mutation of the six-channel artifact is
rejected. Source, class, and signature drift also fail closed. The producer and
verifier share the repository's parser and structural derivation code, so this
is independent replay from source rather than an independently implemented
checker or formal proof of the Rust implementation.

## Reproduction

```console
scripts/run-btor2-symbolic-class-probe-v1.sh /tmp/result.csv
scripts/check-btor2-symbolic-class-probe-v1.sh /tmp/result.csv
scripts/run-btor2-symbolic-class-certificate-probe-v1.sh /tmp/certificate.csv
scripts/check-btor2-symbolic-class-certificate-probe-v1.sh /tmp/certificate.csv
cargo test --locked --test opentitan_pwm_symbolic_class_api
```

## Interpretation

This closes the previous input-free-fixture weakness. It establishes a useful
self-service firmware contract boundary, but not novelty. Shared-input symmetry,
orbit representatives, guarded equivalence predicates, and proof certificates
for finite-state exploration all have close prior art. The next mechanism must
bind an independently checked conditional-equivalence artifact to these exact
source bytes, reuse an expensive property proof only within admitted classes,
and route every singleton or rejected class through exact monolithic checking.
