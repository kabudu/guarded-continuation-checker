# OpenTitan PWM channel-family orbit probe v1

Status: local mechanism gate passes for the identical-binding regime

## Hypothesis

The preceding
[structural negative control](OPENTITAN_PWM_CHANNEL_FAMILY_PROBE_V1.md)
showed that a compact instance map does not reduce exact proof evidence. This
follow-up tests the predeclared next candidate: store one exact certificate per
channel-root equivalence class and derive the corresponding result for every
structurally identical instance.

This is a symmetry optimisation. Symmetry reduction and parameterised
certificates are established prior art, so the result is product-engineering
evidence and not an algorithmic novelty claim.

## Admission invariant

The independent checker derives the orbit rather than trusting a producer
label. It requires:

- at least two instances;
- the same exact channel relation and parameter digest;
- byte-identical ordered core-input and core-root bindings;
- one exact representative certificate for every channel semantic root;
- a feed-forward family grammar in which a channel can name only declared core
  inputs or roots, never another channel's state or expressions; and
- complete reconstruction and source-bound verification of the expanded model.

Every admitted channel therefore starts from the same local state and receives
the same input sequence. Corresponding local states and outputs remain equal by
induction at every frame. A SAFE representative result applies to every member;
an UNSAFE representative input trace reaches the corresponding bad root in
every member at the same frame. Distinct bindings, incomplete root sets,
parameter drift, source drift, mutation, truncation, and resource excess fail
closed.

## Retained result

The exact 15-row result is
[`results/opentitan-pwm-channel-family-orbit-probe-arm64-v1.csv`](../results/opentitan-pwm-channel-family-orbit-probe-arm64-v1.csv),
SHA-256
`055ea9106b1fcfd96db37fb28e065d542e199d824b13c018524c92d9ef24488e`.
No row was discarded.

| Channels | Logical properties | Representatives | Orbit artifact | Direct artifact | Artifact reduction | Orbit evidence | Direct evidence | Evidence reduction |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2 | 10 | 5 | 3,222 B | 5,698 B | 43.453843% | 2,706 B | 5,412 B | 50.000000% |
| 4 | 20 | 5 | 3,834 B | 13,200 B | 70.954545% | 3,186 B | 12,754 B | 75.019602% |
| 6 | 30 | 5 | 4,446 B | 22,622 B | 80.346565% | 3,666 B | 22,016 B | 83.348474% |

All logical SAFE and UNSAFE results and earliest bad frames equal direct exact
replay. Artifacts are byte-identical across five trials at each size. The
logical answer counts are respectively 8 and 2, 16 and 4, and 24 and 6.

Median single-process production times in microseconds were 663 versus 895 at
2 channels, 442 versus 1,354 at 4 channels, and 621 versus 2,861 at 6 channels
for orbit versus direct. Median verification times were 671 versus 864, 451
versus 1,353, and 628 versus 2,860. These small in-process timings exclude
synthesis, process startup, and peak memory, carry no admission threshold, and
must not be generalized to a production workload.

## Decision and next falsification test

The representative format converts structural equality into real retained
evidence and checker-work reduction. It closes the identical-binding mechanism
question that the negative control left open.

It does not close the predeclared OpenTitan subsystem experiment. The current
specialised fixture supplies identical signals to all instances and does not
retain independent per-channel updates or the complete blink, heartbeat,
duty-cycle, and phase-delay state. The next test must build an authentic
multi-channel boundary and partition instances by independently verified
binding equivalence. Singleton or distinct-binding classes must use exact
evidence, and a hidden cross-channel dependency must refuse orbit reuse. That
mixed-orbit result then needs the maintained monolithic baseline, process-level
resource measurements, hosted identity, and clean-checkout acceptance.
