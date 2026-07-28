# OpenTitan PWM exact MMIO-to-RTL mapping v1

Status: translation, independent replay and the predeclared one-phase-cycle
follow-up pass locally. Evidence encoding, maintained comparison, hosted gates
and independent assessment remain open.

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

## Predeclared follow-up result

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

The clean O0 and O2 runs pass all five gates:

- both profiles produce identical semantic replay summaries;
- all 250 invalid inputs still produce zero RTL members;
- all six valid traces contain non-zero PWM observations;
- channel 0 forms one observation class and channels 1 through 5 form a second;
- every trace contains exactly 33 transitions, including the frozen
  17-transition base and the 16-transition quiescent continuation; and
- write-enable or valuation drift in the continuation refuses.

The distinction matches the firmware schedule. Input 0 configures channel 0
twice, ending with duty values 6 and 10. Inputs 1 through 5 leave channel 0 at
duty values 4 and 8 while configuring a disabled selected channel. The result
therefore connects a concrete compiled-firmware choice to a distinct replayed
RTL behavior without granting any RTL member to an invalid input.

This is a positive bounded composition result, not yet a proof-carrying product
artifact. The next gate must encode the mapping and observations, then have a
byte-starting verifier reconstruct them from the firmware certificate, mapping
policy and BTOR2 source.

## Claim boundary

Firmware-aware RTL verification, transaction-level mapping, bounded trace
replay and proof-carrying evidence are established. The positive behavioral
composition is useful integration engineering and a stronger novelty
candidate, but it is not proof that the mechanism is novel. A novelty claim
still requires focused prior-art review, identical-scope maintained-tool
comparison and independent expert assessment.
