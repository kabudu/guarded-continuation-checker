# OpenTitan PWM predicate-to-RTL binding experiment v1

Status: failed at the source-binding gate before evidence production. No RTL
answer or artifact was produced. No novelty or production claim exists.

## Result

The retained `symbolic-class-6.btor2` model cannot satisfy the frozen mapping
contract.

Its source harness exposes two shared configuration classes. Every even
channel consumes class 0 inputs and every odd channel consumes class 1 inputs.
The authentic firmware instead configures exactly the runtime-selected channel
after configuring and enabling channel 0. Consequently:

- channels 0, 2 and 4 collide at the harness boundary;
- channels 1, 3 and 5 collide at the harness boundary;
- selecting one class input also asserts write traffic for untouched sibling
  channels; and
- the hard-coded even-channel values differ from the runtime-selected channel
  configuration for channels 2 and 4.

No complete, semantics-preserving translation exists from the six authentic
MMIO schedules to this harness's six symbolic input bits. Treating the parity
classes as selected channels would invent environmental behavior not present
in the firmware evidence.

The static route therefore refuses before RTL production. Invalid firmware
inputs still create zero RTL members, but none of the six valid inputs is
admitted either.

The next candidate must use a source-attested harness with independently
driven per-channel write enables and complete configuration values. Its
mapping must be derived from the MMIO words rather than channel parity.

## Product question

Can GCC take each independently verified compiled-firmware behavior, translate
its exact ordered MMIO schedule into explicit RTL input valuations, and
independently replay the resulting bounded RTL behavior without granting any
RTL answer to the 250 invalid firmware inputs?

Pairing a firmware certificate with an unrelated RTL proof is insufficient.
The evidence must prove the connection between the exact firmware writes and
the values applied at the RTL boundary.

## Frozen scope

The experiment retains:

- the pinned OpenTitan PWM firmware source and O0/O2 RV32IMC images;
- the complete eight-bit runtime-channel domain;
- the version 1 finite-domain predicate certificate and scalar verifier;
- six valid singleton firmware behaviors and one 250-member invalid behavior;
- the source-attested six-channel OpenTitan PWM BTOR2 model;
- the existing channel-region extraction and property-proof mechanisms; and
- exact per-input firmware execution as the fail-closed fallback.

Only the retained bounded PWM schedule is in scope. General device drivers,
arbitrary bus protocols, interrupts, concurrency and unbounded hardware
execution are outside this version.

## Source-bound mapping contract

One canonical mapping contract binds:

- the exact firmware certificate policy and image identity;
- the exact BTOR2 source identity and semantic roots;
- six channel identifiers in canonical order;
- every accepted MMIO operation, offset, width and value;
- the frame at which each write affects each RTL input;
- the initial and terminal observation frames;
- the complete RTL input vector for every frame; and
- the bounded properties read from each target channel.

For the retained caller, a valid class must contain exactly sixteen MMIO
events. Common control events must match byte-for-byte. The selected-channel
events must use the canonical register offsets for exactly one channel in
`0..5`; offset substitution, aliasing or mixed-channel schedules refuse.

The mapping is an explicit integration contract, not a discovered
environmental assumption. Its source bytes and semantic interpretation are
bound into the resulting evidence.

## Intended production

Production starts only after:

1. the predicate certificate is decoded and independently verified from
   caller-owned image and symbol inputs;
2. all six valid singleton behaviors are extracted in canonical channel order;
3. the invalid predicate is confirmed to have a rejecting return and no
   admissible schedule;
4. the mapping contract translates each valid MMIO stream into a complete
   bounded RTL input trace;
5. the source-attested BTOR2 model is independently parsed; and
6. every translated trace is replayed through the exact word-level RTL
   semantics.

The artifact records the mapping identity, firmware certificate identity,
BTOR2 identity, six complete input traces, target observations, answers and
work counters. Invalid firmware inputs contribute no RTL query, trace, answer
or evidence member.

## Independent verification

The verifier starts from artifact bytes plus separately supplied firmware
image, symbols, mapping source and BTOR2 source. It:

- independently verifies the predicate certificate;
- re-extracts every valid MMIO stream;
- reparses and validates the mapping contract;
- reconstructs all six RTL input traces without trusting producer traces;
- directly replays every BTOR2 transition and target observation;
- confirms the invalid predicate creates zero RTL members;
- compares all recorded answers and earliest observation frames; and
- rejects any extra, missing, reordered or substituted class or query.

No firmware class label, mapping digest, RTL proof label or producer answer is
accepted as semantic evidence.

## Static portfolio and failure behavior

The route is chosen from certificate policy, exact mapping shape, source
identity and bounded work before RTL evidence production:

- `predicate-rtl-v1` only when all six valid schedules translate completely
  and the invalid predicate translates to none; or
- the existing complete exact firmware route when the predicate certificate
  is structurally inapplicable.

Failure after `predicate-rtl-v1` selection returns refusal with no firmware or
RTL answer. It never silently changes route. A mapping rejection on otherwise
valid firmware is a refusal, not an exact RTL fallback, because the integration
contract itself is missing or invalid.

## Hostile controls

The matrix must cover:

- firmware certificate mutation, truncation, source and symbol drift;
- omitted, repeated, reordered or substituted valid channels;
- any invalid input receiving a schedule or RTL member;
- operation, offset, width, value and event-order drift;
- register aliasing and one schedule writing two selected channels;
- missing, duplicated, reordered or extra mapping rows;
- mapping version, source identity, frame and bit-position drift;
- incomplete RTL input vectors and unconstrained input bits;
- BTOR2 source, semantic-root, state, transition and property drift;
- trace, answer and earliest-frame substitution;
- route forcing and source replacement after preflight;
- all artifact mutations, truncations, extensions and over-limit counts; and
- each retained exact-state, live-memory and live-slice negative control.

Every hostile case must refuse before any partial answer becomes visible.

## Resource and compatibility policy

All counts are checked before allocation. The outer artifact remains within
64 MiB, each RTL trace remains within the existing 64-frame and 64-input-bit
policies, channels are exactly six and firmware events remain capped at 32.
Preflight, certificate production, firmware verification, mapping, RTL replay,
artifact bytes, wall time and peak RSS are reported separately.

The Rust API and artifact version are experimental until a tagged release
freezes them. Later versions must not reinterpret version 1 bytes.

## Closest work and claim boundary

Firmware-aware hardware verification, hardware-software co-verification,
transaction-level modeling, symbolic execution, bounded model checking,
proof-carrying code and trace validation are established. The research
question is the narrowly bounded, proof-carrying connection from an exact
finite-domain binary transducer into independently replayed RTL evidence.

No novelty claim is permitted without a focused prior-art review,
identical-scope maintained-tool comparison and independent expert assessment.

## Frozen gates and disposition

The cycle passes only if:

1. O0 and O2 yield the same six valid mappings and zero invalid RTL members;
2. all valid schedules agree with exact firmware and native recording;
3. independent verification reconstructs every mapping and RTL transition;
4. retained RTL answers agree with direct exact GCC and a maintained oracle;
5. every hostile control refuses with no partial answer;
6. two clean artifacts per profile are byte-identical;
7. the complete workflow remains within frozen resource policies;
8. hosted Linux reproduces identities, answers and resource bounds;
9. macOS, Linux and Windows public API checks pass;
10. exact fallback and all earlier negative controls remain unchanged; and
11. prior-art comparison and independent assessment remain open unless
    separately completed.

Gate 1 fails because the retained RTL boundary collapses three firmware
channels into each parity class. Gates 2 through 11 are not attempted. This
failure is retained as a regression guard against presenting grouped symbolic
inputs as exact per-channel firmware evidence.
