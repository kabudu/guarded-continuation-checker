# OpenTitan PWM symbolic property production preflight v1

## Product boundary

The canonical property portfolio already bounded each exact member and the
final artifact. It did not govern the combined production batch before the
first solver began. A valid request could therefore complete several expensive
members before a later member or the final encoder refused the batch.

Production preflight v1 adds a caller-governed, timing-free admission boundary.
It authenticates the structural artifact against separately supplied BTOR2,
validates the complete ordered query set, constructs every representative
property model, selects every exact backend, and totals a deterministic static
work projection before any property solver starts.

The public boundary consists of:

- `Btor2ChannelPropertyProductionPolicy`, with checked artifact and aggregate
  work limits;
- `Btor2ChannelPropertyProductionPlan`, with logical query, proof member,
  backend route, and projected-work counts;
- `preflight_btor2_channel_property_proof`, for admission without solving; and
- `produce_btor2_channel_property_proof_bytes_with_policy`, which performs the
  same admission and emits no partial artifact on refusal.

The inclusive boundary is intentional. A plan is admitted at its exact
projected-work value and refused one unit below it. Query and proof-member
limits are also checked during preflight. Invalid structural evidence remains
an error and cannot trigger an ungoverned fallback.

## Meaning of projected work

Projected work is a deterministic admission token, not elapsed time, peak
memory, or an exact instruction count. Explicit-state members use a
source-derived valuation, node, state, and layer projection. Bitblast members
use a source-derived node-width-squared and frame-scaled projection. The two
routes intentionally share one policy dimension so a caller can bound batch
fanout without machine-specific calibration.

Each backend retains its existing hard per-member limits for horizon, input
width, generated variables, clauses, and certificate bytes. The aggregate
projection complements those limits. It does not replace process deadlines,
address-space enforcement, measured peak memory, or output caps at the future
CLI boundary.

## Retained matrix

The predeclared 2, 4, and 6-channel models are planned at horizons 1 and 2.
Every row is accepted at its exact limit and refused one unit below it.

| Channels | Horizon | Queries | Members | Explicit | Bitblast | Projected work |
|---:|---:|---:|---:|---:|---:|---:|
| 2 | 1 | 4 | 4 | 4 | 0 | 1,251,200 |
| 2 | 2 | 4 | 4 | 0 | 4 | 1,918,560 |
| 4 | 1 | 8 | 6 | 6 | 0 | 4,008,000 |
| 4 | 2 | 8 | 6 | 0 | 6 | 4,515,984 |
| 6 | 1 | 12 | 6 | 6 | 0 | 6,937,920 |
| 6 | 2 | 12 | 6 | 0 | 6 | 6,189,840 |

The horizon-2 six-channel value is lower than the horizon-1 value because the
static router changes backend. Projected values compare admission cost within
the versioned projection contract; they are not benchmark timings.

## Reproduction

```console
scripts/run-btor2-symbolic-property-preflight-probe-v1.sh /tmp/result.csv
scripts/check-btor2-symbolic-property-preflight-probe-v1.sh /tmp/result.csv
cargo test --release --locked --test opentitan_pwm_symbolic_class_api aggregate_production_preflight
```

The retained arm64 result selects all six predeclared rows and discards none.
No timing or process-memory conclusion is drawn from it.

## Remaining gates

- Add a strict, bounded, no-clobber file CLI and typed process client.
- Enforce deadlines, process memory, output size, and process-tree containment
  around production and verification.
- Retain per-phase observability and measured peak resources.
- Reproduce deterministic artifacts and behavior on Linux, macOS, and Windows.
- Compare realistic safety properties with maintained tools and obtain
  independent operator review.

This is resource-governance engineering around established exact backends. It
does not change the novelty boundary.
