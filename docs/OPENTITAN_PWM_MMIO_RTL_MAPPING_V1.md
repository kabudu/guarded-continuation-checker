# OpenTitan PWM exact MMIO-to-RTL mapping v1

Status: translation and independent replay pass locally. The first observation
window is non-discriminating, so no RTL property, novelty or production claim
results.

## Question

Can GCC translate every independently verified valid compiled-firmware MMIO
schedule into complete inputs for the source-attested per-channel RTL model,
while producing no RTL member for any invalid firmware input?

## Frozen mapping

The mapper accepts only the retained 16-event schedule. It checks every
operation, offset, value and position, including the selected channel encoded
by consecutive duty-cycle and parameter register offsets. It normalizes
16-bit phase-tick fields into the four-bit beat domain only when division by
4,096 is exact and the result remains in `0..15`.

Each trace contains one complete initial valuation followed by one complete
valuation per MMIO event. Register write enables pulse only on their
corresponding event. Reads and the terminal observation retain values without
asserting a write.

The invalid predicate must have the exact ten-event rejecting prefix for all
250 inputs in `6..255`. It produces zero RTL traces, transitions, observations
and answers.

## Independent replay

The replayer starts from the pinned 97,036-byte BTOR2 source with SHA-256
`159adf69ab636d95195b2a65dd5d7afd46f05b3bad8326659fe07943cd425f7b`.
It reparses the model, requires all 40 source inputs and the 39-input semantic
support, binds values by exact source symbol and width, reconstructs the
initial state and evaluates every transition. Producer state and observations
are not accepted as inputs.

## Local result

Pinned O0 and O2 RV32IMC builds pass independently:

| Profile | Firmware verifier transitions | Lane operations | Valid RTL members | Invalid RTL members | RTL transitions |
| --- | ---: | ---: | ---: | ---: | ---: |
| O0 | 12,781 | 303,250 | 6 | 0 | 102 |
| O2 | 4,408 | 112,000 | 6 | 0 | 102 |

Both profiles produce byte-identical semantic RTL summaries. Every trace
advances the harness step from 0 through 15, but all 18 recorded six-channel
PWM observations remain zero. The mapping and replay mechanism therefore
passes, while the useful-observation gate fails.

## Predeclared follow-up

The next cycle appends exactly 16 quiescent transitions after the canonical
event schedule. Sixteen is one complete cycle of the harness's source-declared
four-bit phase counter, not a per-formula calibrated horizon. All write enables
remain zero and all register values remain fixed during the continuation.

The follow-up passes its observation gate only if:

1. O0 and O2 remain semantically identical;
2. all 250 invalid inputs still produce zero RTL members;
3. at least two valid channel traces have different non-empty PWM observation
   sequences;
4. independent replay reconstructs every transition from source and MMIO
   inputs; and
5. model, schedule, valuation or observation drift refuses.

If one complete phase cycle remains non-discriminating, this candidate is
retained negative without extending or tuning the horizon.

## Claim boundary

Firmware-aware RTL verification, transaction-level mapping, bounded trace
replay and proof-carrying evidence are established. This result is useful
integration engineering, not evidence that the mechanism is novel. A novelty
claim still requires focused prior-art review, identical-scope maintained-tool
comparison and independent expert assessment.
