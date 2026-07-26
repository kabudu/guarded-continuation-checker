# OpenTitan PWM guarded channel-relation probe v1

## Outcome

The frozen single-positive-input guard hypothesis is negative on the authentic
six-channel OpenTitan PWM boundary.

All twelve mixed-class claims are UNSAFE by horizon 2. The probe checks
channels 0 and 1 under both equality and difference, guarded by each bit of
`class_write_i`, `class_invert_i` and `class_enable_i`. Independently verified
bitblast certificates provide concrete counterexamples at frames 0 through 2.

| Queries | SAFE | UNSAFE | Certificate bytes per query | Verified |
|---:|---:|---:|---:|---:|
| 12 | 0 | 12 | 125 | 12 |

## Interpretation

A current-frame positive firmware input does not capture the configuration
state accumulated by earlier writes. It therefore cannot justify a
mixed-class output relation at the same frame. Both equality and difference
can be falsified while any one selected input bit is true.

This rejects the version-1 guard language for this source. GCC must not infer a
stronger guard from the observed counterexamples or expected answers.

The next admissible research direction is a separately predeclared
state-bearing guard language, such as a source-derived monitor that binds the
relevant write history. It must still be checked against direct product
queries and must account for the monitor certificate in every size and work
comparison.

## Reproduction

```sh
cargo run --release --example btor2_guarded_channel_relation_probe -- \
  /tmp/btor2-guarded-channel-relation-probe-v1.csv
cmp \
  results/opentitan-pwm-guarded-channel-relation-probe-v1.csv \
  /tmp/btor2-guarded-channel-relation-probe-v1.csv
```
