# Roa Logic PLIC retained revision cost v1

## Scope

This controlled workload measures the regime where a public embedded component
remains unchanged while a small GCC-authored monitoring component is revised.
It complements, but does not replace, the authentic PLIC source-revision
cohort. The workload is designed to expose whether the retained API actually
removes unchanged proof work. It is not an upstream PLIC performance claim.

The unchanged left component is the later pinned Roa Logic PLIC gateway model.
The right component changes from a one-frame pending monitor to a persistent
pending monitor. Both monitors, their properties, and the one-bit wiring
contract are repository-authored.

## Method

The release-mode benchmark alternates full and retained production for 21
trials, then compares from-scratch and retained verification. It checks before
timing that:

- both production paths return the same bounded answer;
- both paths emit byte-identical complete artifacts;
- the ordinary verifier accepts the full artifact; and
- the retained verifier accepts the same artifact after validating the PLIC
  relation once.

Reproduce with:

```sh
cargo run --release --locked --example roalogic_plic_revision_cost -- \
  PLIC.btor2 \
  corpus/rtl/roalogic-plic-gateway/revision-cohort/monitor.btor2 \
  corpus/rtl/roalogic-plic-gateway/revision-cohort/monitor-v2.btor2 \
  corpus/rtl/roalogic-plic-gateway/revision-cohort/interface-retained-plic.txt
```

## Local arm64 result

[`roalogic-plic-revision-cost-arm64-v1.csv`](../results/roalogic-plic-revision-cost-arm64-v1.csv)
records one 21-trial run with Rust 1.97.0:

- full production covers 4,100 complete local candidate valuations;
- retained production covers four changed-side valuations, produces one local
  section, and reuses one validated section;
- both paths emit the same 525,095-byte artifact;
- retained median production is 7.12% of full production; and
- retained median verification is 13.45% of from-scratch verification.

Timing is observational and machine-dependent. Backend selection never uses
these values. The deterministic evidence is the 4,100-to-4 work reduction,
one-produced/one-reused accounting, exact answer agreement, and byte-identical
artifact.

## Remaining baseline work

Hosted amd64 replication, whole-process peak resident memory, repeated process
amortisation, and the closest composed-witness comparison remain open. The
result does not yet close the predeclared strong-baseline or novelty gates.
