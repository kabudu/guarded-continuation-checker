# OpenTitan PWM channel phase-abstraction probe v1

## Outcome

The fixed ten-bit phase abstraction is smaller than the 69-bit concrete source
state, and its non-vacuity mechanism works. It does not preserve the selected
mixed-class relation.

Twenty-five of 27 phase-and-history guards carry independently verified
reachability witnesses. Two phase-zero written-history guards are refused
because their unreachability proofs exceed the governed certificate limit.

Of 54 relation rows, 46 have verified counterexamples and eight return `NONE`
after proof refusal or missing non-vacuity evidence. There are no accepted SAFE
relations.

Most decisively, every phase from 2 through 8 admits concrete counterexamples
to both equality and difference under every frozen history guard. The same
ten-bit abstract class therefore contains both Boolean relation values.

| Rows | Reachable | Relation UNSAFE | NONE | Accepted SAFE |
|---:|---:|---:|---:|---:|
| 81 | 25 | 46 | 10 | 0 |

## Interpretation

Shared phase plus firmware configuration history remains too coarse. The
missing discriminator is channel-local operational state. Adding enough such
state to make the relation functional risks recreating the concrete product,
which the predeclared complete-state control forbids treating as an
abstraction.

This falsifies the phase abstraction before portfolio, fallback or maintained
baseline work. The result is semantic, not merely operational: the accepted
UNSAFE certificates demonstrate both relation values inside the same abstract
class.

## Reproduction

```sh
cargo run --release --example btor2_channel_phase_abstraction_probe -- \
  /tmp/btor2-channel-phase-abstraction-probe-v1.csv
cmp \
  results/opentitan-pwm-channel-phase-abstraction-probe-v1.csv \
  /tmp/btor2-channel-phase-abstraction-probe-v1.csv
```
