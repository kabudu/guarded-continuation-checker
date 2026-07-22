# OpenTitan prim_count distinct-property query service v1

Status: measured mechanism result with a disconfirming maintained-system
comparison. Not a novelty or production-readiness claim.

## Question

Can a service validate the unchanged environment and one revision-specific
OpenTitan `prim_count` relation once, then answer several distinct properties
without rebuilding either local relation? Does that provide a defensible
advantage over a maintained proof-producing model checker at the same property
scope?

The workload contains eight predicates per revision: count equals 0, 1, 2, or
3; the error flag; both count bits; and count parity. This replaces an invalid
earlier design that repeated one predicate at several horizons. A single
unbounded proof could have answered that earlier workload, so it was rejected
before publication.

## Integrity result

The service holds validated environment and counter relations in memory. Every
new interface contract selects one of the eight projected property roots. The
normal bounded producer independently rebuilds both relations for every query;
the retained producer rebuilds neither relation after initial admission.

Across both semantic revisions and all 16 queries:

- retained and full production emit byte-identical certificates;
- the old revision returns seven SAFE and one UNSAFE answer;
- the new revision returns five SAFE and three UNSAFE answers;
- only three local sections are produced and 29 are reused;
- retained verification reuses 31 already validated sections; and
- enumerated local candidate work falls from 655,424 to 81,924, a reduction of
  87.5006%.

Five arm64 trials give median retained/full ratios of 0.213828 for production
and 0.071409 for verification. These ratios compare two paths in GCC's own
engine. They do not compare GCC with another solver.

## Maintained-tool comparison

Pinned Yosys creates one AIGER model for each revision and property. Qualified
rIC3 produces 12 SAFE witnesses and four UNSAFE traces. Qualified Certifaiger
checks every witness or trace independently. All 16 answers agree with GCC.

The complete AIGER model set is 4,935 bytes and its evidence is 3,957 bytes.
The GCC certificates total 113,264,568 bytes, about 12,738 times the maintained
model-plus-evidence size. The recorded three-second producer and five-second
checker measurements include 32 isolated Docker launches, while GCC timings
are in-process nanosecond measurements. They are not a defensible runtime
comparison.

This result falsifies a general artifact-efficiency claim for the current
container. Retaining relations saves internal recomputation, but every final
certificate embeds both shared relations again. The next justified experiment
is a canonical content-addressed batch certificate: store each validated local
relation once, bind every query to those shared sections by digest, and carry
only its interface, composition, and bounded answer. Independent verification
must reject missing, substituted, reordered, duplicated, or unreferenced
sections and reproduce every existing standalone certificate's answer.

## Reproduction

Build the GCC measurement binary and run five trials:

```console
cargo build --release --locked --example opentitan_prim_count_query_service
mkdir /tmp/gcc-query-service
TRIALS=5 scripts/benchmark-opentitan-prim-count-query-service-v1.sh \
  /path/to/pinned/yosys \
  target/release/examples/opentitan_prim_count_query_service \
  /tmp/query-service.csv /tmp/gcc-query-service
```

Run the maintained baseline with the already qualified tools:

```console
mkdir /tmp/gcc-query-baseline
scripts/benchmark-opentitan-prim-count-query-baseline-v1.sh \
  /path/to/pinned/yosys /path/to/ric3-output \
  /path/to/certifaiger-output /tmp/query-baseline.csv \
  /tmp/query-baseline.manifest.txt /tmp/gcc-query-baseline 1
```

The retained arm64 observations are in
`results/opentitan-prim-count-query-service-arm64-v1.csv`. The equivalent
maintained results and tool digests are in
`results/opentitan-prim-count-query-baseline-arm64-v1.csv` and its manifest.
