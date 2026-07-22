# OpenTitan PWM channel-family certificate v1

Status: predeclared and prior-art bounded, not implemented or measured

## Question

Can GCC certify one source-bound `pwm_chan` transition relation, instantiate
that checked relation across several identical PWM channels using explicit
state and port renaming, and compose the instances with one shared `pwm_core`
relation without duplicating the channel proof?

The result must preserve the same bounded answers and reconstructable traces as
a monolithic whole-subsystem model. It must fail closed to exact monolithic
verification when source identity, parameters, instance wiring, or independence
conditions do not match.

This is relevant to embedded and robotics systems that repeat one verified IP
block across PWM outputs, motor axes, sensor lanes, DMA channels, interrupt
sources, or redundant monitors.

## Pinned public source

The first cohort uses the same OpenTitan crosstalk revision already retained by
the two-atom experiment:

- child commit `86db2898288664d8d5e8fc635b48951ef63e3439`;
- parent commit `376021484b3cab4ef0d352f73d16f0b7a80c0970`;
- `hw/ip/pwm/rtl/pwm_core.sv`;
- `hw/ip/pwm/rtl/pwm_chan.sv`;
- exact upstream licence and retained mail patch.

The parent and child files, their existing SHA-256 values, and the pinned Yosys
revision remain unchanged. The experiment may provide a minimal generated
`pwm_reg_pkg` type boundary and top-level stimulus wrapper, but it may not
replace the selected core or channel transition equations with a behavioural
model.

## Frozen subsystem boundary

The subsystem has one shared PWM core and repeated channel instances at three
sizes:

- 2 channels;
- 4 channels;
- 6 channels, matching OpenTitan's default `NOutputs`.

Width specialisation may reduce phase and beat counters only if the exact
source equations and overflow behaviour remain intact. Each admitted model
must retain:

- the shared beat and phase counter state;
- per-channel enable and inversion capture;
- per-channel clear selection;
- each channel's blink and heartbeat state;
- duty-cycle, phase-delay, wrap, enable, and inversion logic;
- the parent combinational or child registered output equation selected by the
  source revision.

Yosys hierarchy and state reports must prove the expected number of channel
instances and named state families. A wrapper that collapses the repeated
channels to one shared state vector is rejected.

## Candidate certificate

The candidate artifact contains:

1. one source-bound shared-core relation and its independent completeness
   evidence;
2. one source-bound channel-family relation and its independent completeness
   evidence;
3. a canonical instance table containing the instance identifier, parameter
   values, input and output port maps, and a disjoint state-renaming map;
4. composition evidence for the core-to-channel and channel-to-output wiring;
5. bounded query answers plus reconstructable SAFE or UNSAFE evidence;
6. the exact fallback result for any query that the family route refuses.

The verifier must validate the core and family evidence before instantiation,
prove that every state-renaming range is injective and pairwise disjoint, bind
every port exactly once, reconstruct each instantiated transition relation, and
independently replay the final queries. It must not trust producer-supplied
claims of symmetry, independence, or answer equivalence.

## Prior-art boundary

The pre-implementation search rejects the broad novelty hypothesis. Symmetry
quotients for repeated processes, guarded annotated quotient structures that
recover original transitions, and independently checked proof certificates for
systems with an arbitrary number of components are prior art. The detailed
sources and consequences are recorded in the
[repeated channel-family reassessment](PRIOR_ART_AUDIT_V1.md#repeated-channel-family-reassessment).

This experiment therefore tests an engineering and product hypothesis, not a
new-algorithm claim. Its candidate distinction is the combination of a finite
RTL source boundary, canonical injective instance maps, independent expanded
model authentication, bounded SAFE and UNSAFE evidence, reconstructable traces,
hostile wiring controls, and exact monolithic fallback. These differences do
not become evidence of novelty merely because the implementation or format is
different.

## Frozen revision atoms

The authentic source-change boundary remains:

1. `core-clear`: parent or child shared-core clear selection;
2. `channel-output`: parent or child channel-family output registration.

The channel change is one source atom instantiated repeatedly, not one invented
source atom per channel. The four source combinations therefore remain masks
0 through 3. Per-instance fault injection is a separate environment dimension
and must not be reported as an upstream source revision.

## Frozen query families

For every channel count, the cohort includes:

- a channel-local enable or inversion update that may restart only the selected
  channel;
- an unrelated-channel update that must not restart a running neighbour;
- a raw-output transition that distinguishes parent combinational output from
  child registered output;
- a joint clear-plus-output transition whose SAFE result requires both source
  atoms;
- pairwise output noninterference for every adjacent channel pair;
- simultaneous updates to the first and last channel;
- reset-low SAFE controls for every output;
- one unchanged UNSAFE control.

The exact property identifiers and horizons must be retained before the first
candidate certificate is produced. At least one query must reconstruct a trace
that crosses the shared core into two different channel instances.

## Admission and fallback

Family instantiation is admitted only from static structure:

- identical channel source digest and parameters;
- canonical interface schema;
- no cross-instance state alias;
- no channel-to-channel combinational or sequential edge except through a
  declared shared-core port;
- supported transition operators and reset semantics;
- complete independently checkable core and family evidence;
- resource limits that cover expanded relation size, query count, evidence
  bytes, verifier work, and trace reconstruction.

Admission uses no timing observation and no per-formula calibration. A refused
candidate runs the existing exact whole-model path. Resource exhaustion,
malformed evidence, unsupported operators, ambiguous wiring, or any verifier
disagreement fails closed rather than silently downgrading a claimed family
certificate.

## Hostile matrix

The independent verifier must reject at least:

- duplicate instance identifiers;
- overlapping renamed state ranges;
- omitted or duplicate port bindings;
- swapped channel output bindings;
- mismatched parent and child source digests;
- one altered parameter in an otherwise identical family;
- an undeclared cross-channel edge;
- reordered instances that violate canonical encoding;
- a changed query horizon or property identifier;
- altered core, family, composition, answer, or trace evidence;
- excessive instance, relation, query, or evidence counts;
- truncated and trailing bytes.

## Baselines

The closest maintained route uses the same pinned source, Yosys, rIC3, and
Certifaiger toolchain. It must:

- build monolithic 2, 4, and 6-channel models;
- agree on every source combination and query;
- independently validate every invariant and trace;
- run in the qualified single-container sequential and fixed four-way modes;
- retain complete model and evidence bytes, synthesis time, producer and
  checker wall time, and clearly scoped resource measurements.

The closest GCC baseline is its own monolithic exact path with no family-proof
reuse. A maintained persistent service is added if the selected tool exposes a
real supported service mode. Fresh processes in one container must not be
described as a warm service.

## Predeclared gates

The mechanism advances only if all gates pass:

1. exact upstream provenance and structural hierarchy attestation;
2. identical monolithic and family-composed answers for every row;
3. independently checked SAFE and UNSAFE evidence;
4. reconstructable traces with exact instance and source bindings;
5. deterministic bytes across two clean productions;
6. exact fail-closed fallback for every refused case;
7. the complete hostile matrix;
8. explicit producer and verifier resource governance;
9. five clean trials at each channel count for GCC monolithic, GCC family, and
   both maintained orchestration modes;
10. retained artifact size, wall time, peak RSS scope, and checker work without
    a performance pass threshold;
11. hosted Linux release-build reproduction;
12. frozen-format verification on Linux, macOS, and Windows;
13. a closest-prior-art update before any novelty wording;
14. an external-style self-service acceptance run from a clean checkout.

No trial may be discarded selectively. A harness or provenance defect requires
the complete affected matrix to be rerun and the reason to be documented.

## Decision rule

The mechanism succeeds if one independently proved channel relation can be
safely instantiated and composed while exact whole-model verification agrees
on every answer and trace.

A practical reuse claim additionally requires the family artifact or verifier
work to grow more slowly than duplicated per-channel evidence at 2, 4, and 6
channels. A novelty claim remains prohibited. Passing the current gates can
establish a useful source-bound family artifact, but the audit already
disconfirms novelty of the broad repeated-instance and parameterised-certificate
mechanism. Only a subsequent, separately predeclared invariant or capability
that is absent from the closest systems could reopen that question. Negative
size, time, memory, or prior-art results are retained with the same prominence
as positive results.
