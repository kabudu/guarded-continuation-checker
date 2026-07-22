# Revision batch certificate v1

Status: implemented experimental typed API with local public-design evidence.
Not a production-supported format or a novelty claim.

## Purpose

A revision-local standalone certificate embeds its left and right relations.
That is useful for independent exchange, but it repeats large byte-identical
sections when a service answers several properties over the same components.
The OpenTitan distinct-property workload repeated 113,264,568 certificate bytes
across 16 answers.

Revision batch certificate v1 separates shared local relations from
query-specific evidence. Each relation appears once in strict evidence-digest
order. Every entry addresses its left and right relations by SHA-256 digest and
carries its canonical source-bound interface plus bounded answer. All sections
must be referenced. Entries are strictly ordered by their complete canonical
encoding.

The producer enumerates and validates every component relation once. The
independent verifier accepts the original component sources and encoded batch,
checks that the source set exactly matches the shared sections, semantically
replays each local relation once, recomposes every interface, and verifies every
bounded answer. A separate extraction API reconstructs the ordinary standalone
certificates without changing a byte of their four sections.

## Security and integrity properties

The current tests reject:

- every truncated prefix and trailing data;
- oversized section and count declarations before allocation;
- corrupted section contents;
- missing, duplicated, reordered, or unreferenced shared sections;
- duplicated or reordered entries;
- missing, duplicate, or surplus verifier sources; and
- a known-section substitution even after the attacker restores canonical
  entry order.

The format has a 64 MiB total limit, at most 64 shared components and at most
256 entries. Existing local, interface and final-section limits remain in
force. V1 uses SHA-256 content addressing. Its security assumes collision
resistance and does not provide signatures, author identity, freshness, or
release-policy admission.

## OpenTitan result

The batch contains the environment, old counter and new counter relations once,
then binds eight distinct property queries to each revision. Across five arm64
trials it has deterministic size and answers:

- 16 extracted standalone certificates: 113,264,568 bytes;
- one shared-section batch: 14,164,144 bytes;
- bytes removed: 99,100,424, or 87.4946%;
- shared sections: three;
- local candidate valuations: 81,924;
- answers: 12 SAFE and four UNSAFE; and
- every extracted standalone certificate is byte-identical to independent
  production.

Median batch production is 405,916,500 ns and median independent verification
is 270,890,000 ns on the reference arm64 development host.

This repairs the measured duplication defect, but it does not beat the closest
maintained evidence route. The equivalent AIGER models plus rIC3 and Certifaiger
evidence occupy 8,892 bytes, so the batch remains about 1,593 times larger. The
result is useful proof-carrying service engineering. Content addressing,
deduplication, compositional verification and proof caching are established
techniques, so no algorithmic novelty follows from their combination here.

## Reproduction

```console
cargo build --release --locked --example opentitan_prim_count_revision_batch
mkdir /tmp/gcc-revision-batch
TRIALS=5 scripts/benchmark-opentitan-prim-count-revision-batch-v1.sh \
  /path/to/pinned/yosys \
  target/release/examples/opentitan_prim_count_revision_batch \
  /tmp/gcc-revision-batch.csv /tmp/gcc-revision-batch
```

The retained arm64 observations are in
`results/opentitan-prim-count-revision-batch-arm64-v1.csv`.
