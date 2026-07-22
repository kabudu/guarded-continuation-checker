# OpenTitan PWM symbolic property codec v1

## Product boundary

Property portfolio v2 was an exact typed Rust value, not an exchangeable
artifact. Codec v1 adds the canonical outer boundary needed to persist, move,
archive and independently verify the complete portfolio.

The `GCCBCP01` envelope binds:

- format version and source SHA-256;
- the complete canonical structural admission artifact;
- every ordered channel, property and horizon query;
- every canonical class member, representative, route and exact certificate;
- aggregate evidence and artifact resource limits; and
- a SHA-256 envelope checksum.

Public byte APIs produce, encode, decode and source-verify the complete
portfolio. Decoder policy independently limits query count, member count,
aggregate evidence and total bytes. The decoder checks the envelope checksum,
version and structural source binding before query or evidence allocation. It
then preflights every count and solver-specific certificate length, enforces
canonical member ordering, decodes each nested certificate under its own
limits, and requires byte-identical re-encoding. Semantic verification still
reconstructs the property models from separately supplied BTOR2 and replays the
structural admission and every exact proof.

## Compatibility contract

The six-channel horizon-2 artifact is frozen at 1,568 bytes with SHA-256:

```text
31db59025d13872959c11783d6f1887fd98f3bac9e0234f3da7fb88ed52e3486
```

An executable downstream test regenerates that identity, round-trips it, and
verifies all twelve logical answers. Any future wire change requires a new
magic/version pair or an explicit decoder migration path. Silent
reinterpretation of v1 bytes is prohibited.

Every truncation and every single-byte mutation of this retained artifact is
rejected. Tests also reject trailing bytes, source and query drift, and caller
limits below each retained query, member, evidence or artifact dimension.
Checksum-refreshed attacks against the version, nested structural artifact,
counts, member ordering, solver tag and evidence length are rejected by their
own structural checks rather than relying on checksum failure.

## Retained result and negative control

| Structural admission | Member evidence | Complete artifact | Twelve direct witnesses | Complete versus direct |
|---:|---:|---:|---:|---:|
| 460 B | 750 B | 1,568 B | 1,500 B | +4.53% |

The earlier 19.33% evidence reduction remains correct for structural admission
plus retained member evidence. Once the new outer metadata and checksum are
included, the complete portable artifact is 4.53% larger than twelve raw direct
witness certificates. This is a deliberate retained negative result. Codec v1
closes integrity and compatibility mechanics, not a complete-artifact size win.

## Reproduction

```console
scripts/run-btor2-symbolic-property-codec-probe-v1.sh /tmp/result.csv
scripts/check-btor2-symbolic-property-codec-probe-v1.sh /tmp/result.csv
cargo test --release --locked --test opentitan_pwm_symbolic_class_api outer_property_portfolio
```

## Remaining gates

- Add a strict, no-clobber file CLI and typed process client around the byte
  APIs for self-service embedded build integration.
- Preflight aggregate production work before any solver member starts.
- Add generation and verification observability, deadline and process resource
  enforcement, and measured peak memory.
- Cross-check realistic safety properties and a broader operator corpus against
  maintained tools.
- Reproduce the frozen bytes on Linux, macOS and Windows and retain a tagged
  compatibility history.
- Obtain independent implementation and suitability review.

The codec is conventional canonical artifact engineering. It does not alter
the current novelty boundary.
