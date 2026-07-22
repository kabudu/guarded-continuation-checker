# Authentic OpenTitan PWM channel-family source boundary

This fixture retains the exact OpenTitan PWM child sources at commit
`86db2898288664d8d5e8fc635b48951ef63e3439` and synthesises the complete core
plus 2, 4, and 6 complete `pwm_chan` instances.

The authored harness supplies deterministic register traffic with two channel
configuration classes. It reduces counter widths only through the upstream
module parameters. It does not replace blink, heartbeat, duty-cycle,
phase-delay, wrap, enable, inversion, clear, or registered-output equations.

Pinned Yosys cannot consume the generated `pwm_reg2hw_t` structure. The build
therefore authenticates the full package and mechanically lowers only the
core's register-structure field references to width-equivalent ports. It
rejects an unrecognised reference. The upstream core and channel files remain
unaltered in `upstream-child/`.

Regenerate to new paths with:

```console
scripts/build-opentitan-pwm-authentic-channel-family-v1.sh \
  /path/to/pinned/yosys \
  /tmp/authentic-2.btor2 \
  /tmp/authentic-4.btor2 \
  /tmp/authentic-6.btor2
```

The retained models are property-free source boundaries for the extraction
experiment. They are not yet proof artifacts or product-support evidence.
