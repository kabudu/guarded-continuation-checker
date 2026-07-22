# OpenTitan PWM channel-family preliminary probe v1

Status: completed negative control; not the predeclared full subsystem result

## Scope

This probe measures the first source-bound family artifact and exact proof
portfolio against GCC's direct exact route. It reuses the retained OpenTitan PWM
crosstalk components at child revision
`86db2898288664d8d5e8fc635b48951ef63e3439` and expands the channel relation at
2, 4, and 6 instances. Every size is produced and verified five times in one
release-build process.

The probe is deliberately narrower than the
[channel-family experiment](OPENTITAN_PWM_CHANNEL_FAMILY_CERTIFICATE_V1.md).
All instances receive identical core bindings, and the retained specialised
channel contains the earlier pulse and registered-output equations rather than
the complete blink, heartbeat, duty-cycle, phase-delay, and independent
per-channel configuration boundary. It therefore cannot establish neighbour
noninterference, authentic multi-channel hierarchy, maintained-tool parity, or
the final product claim.

## Retained result

The exact 15-row result is
[`results/opentitan-pwm-channel-family-probe-arm64-v1.csv`](../results/opentitan-pwm-channel-family-probe-arm64-v1.csv),
SHA-256
`02168b21c803e1f2143059401023b7b7f2e7b1a19817a168c27b54651d9e056a`.
No row was discarded.

| Channels | Properties | Family map | Expanded model | Map reduction | Family portfolio | Direct portfolio | Family premium | Exact evidence |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2 | 10 | 356 B | 1,596 B | 77.694236% | 5,990 B | 5,698 B | 5.124605% | 5,412 B |
| 4 | 20 | 488 B | 2,754 B | 82.280320% | 13,584 B | 13,200 B | 2.909091% | 12,754 B |
| 6 | 30 | 620 B | 3,918 B | 84.175600% | 23,098 B | 22,622 B | 2.104146% | 22,016 B |

Every family answer and reconstructed trace summary equals the direct exact
result. Each channel count has byte-identical family and direct artifacts over
all five trials. SAFE and UNSAFE counts are respectively 8 and 2, 16 and 4,
and 24 and 6.

Median single-process production times in microseconds were 502 versus 438 at
2 channels, 1,532 versus 1,402 at 4 channels, and 3,000 versus 2,967 at 6
channels for family versus direct. Median verification times were 514 versus
445, 1,560 versus 1,542, and 3,058 versus 2,881. These sub-millisecond to
low-millisecond measurements are descriptive only. They exclude source
synthesis, process startup, and process-level peak memory and are not an
admission threshold.

## Decision

Structural family encoding works: the instance map grows more slowly than the
fully expanded model. The proof breakthrough does not. Family and direct routes
carry exactly the same search evidence, so the compact map adds 292 to 476
bytes to the complete portfolio and does not reduce independent checker work.

This falsifies the idea that model expansion alone creates proof reuse. The
next bounded candidate is a representative-orbit evidence format that stores
one checked query certificate per structurally proven equivalence class and
independently validates every instance-to-representative map. Symmetry reduction
is established prior art, so success would be an engineering optimisation, not
a novelty result. The route must refuse distinct bindings or hidden coupling
and preserve exact monolithic fallback.

That candidate is evaluated in the
[channel-family orbit probe v1](OPENTITAN_PWM_CHANNEL_FAMILY_ORBIT_PROBE_V1.md).
