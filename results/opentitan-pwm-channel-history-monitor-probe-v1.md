# OpenTitan PWM channel history-monitor probe v1

## Outcome

The frozen six-bit firmware write-history monitor does not establish a
persistent mixed-class relation on the authentic OpenTitan PWM boundary.

The complete 30-row cohort contains eight SAFE and 22 UNSAFE results. Every
query is answered and independently verified by the proof-carrying bitblast
backend.

| Horizons | Guards | Relations | Queries | SAFE | UNSAFE | Refused |
|---|---:|---:|---:|---:|---:|---:|
| 0, 1, 2, 4, 8 | 3 | 2 | 30 | 8 | 22 | 0 |

All six horizon-2 queries are UNSAFE. At horizon 8,
`SameTrackedConfig` and `OppositeTrackedInvert` each admit equality and
difference counterexamples at frame 7.

## Interpretation

Remembering prior firmware writes repairs the temporal defect in a
current-frame input guard, but it still omits operational phase and internal
channel evolution. Equal retained enable and invert configuration does not
make the mixed channel implementations relationally equivalent because their
static phase, duty-cycle, blink and heartbeat parameters differ.

The positive horizon-0 and horizon-1 rows are too shallow to justify a reusable
temporal capability. The monitor therefore fails the predeclared persistence
test and must not be promoted into the proof portfolio.

The next hypothesis must account for operational phase without copying the
complete channel state. A phase abstraction is useful only if its proof and
fallback costs remain below independent direct queries.

## Reproduction

```sh
cargo run --release --example btor2_channel_history_monitor_probe -- \
  /tmp/btor2-channel-history-monitor-probe-v1.csv
cmp \
  results/opentitan-pwm-channel-history-monitor-probe-v1.csv \
  /tmp/btor2-channel-history-monitor-probe-v1.csv
```
