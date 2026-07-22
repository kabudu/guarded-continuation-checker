# OpenTitan PWM crosstalk impact cohort

This corpus specialises the two connected behavioural changes in OpenTitan PWM
commit `86db2898288664d8d5e8fc635b48951ef63e3439`:

- selective per-channel clear handling in `pwm_core.sv`;
- registered glitch-free output in `pwm_chan.sv`.

The generated models preserve four old/new atom combinations and five frozen
query classes. The joint query changes from UNSAFE to SAFE only for changed
mask `3`, meaning both authentic source atoms are required. Masks `1` and `2`
separately explain the core-only and channel-only regressions. Unchanged SAFE
and UNSAFE controls are retained.

Regenerate the canonical models with pinned Yosys:

```console
mkdir /tmp/gcc-pwm-impact
scripts/build-opentitan-pwm-crosstalk-impact-v1.sh \
  /path/to/pinned/yosys \
  /tmp/gcc-pwm-impact/core-before.btor2 \
  /tmp/gcc-pwm-impact/core-after.btor2 \
  /tmp/gcc-pwm-impact/channel-before.btor2 \
  /tmp/gcc-pwm-impact/channel-after.btor2
```

The specialised SystemVerilog, generated BTOR2, interface, queries, source
digests, and derivation boundary are retained. The aggregate certificate is not
committed; its accepted local identity is recorded in the experiment document.
This corpus is mechanism evidence, not complete OpenTitan PWM assurance.
