# OpenTitan PWM authentic channel extraction v1

Status: predeclared, not implemented or measured

## Purpose

The reduced channel-family fixture proved the structural and representative
proof mechanisms, but it did not retain the complete OpenTitan PWM channel.
This experiment tests the next product boundary without replacing upstream
transition equations with a behavioural model.

The candidate must discover a reusable repeated-channel boundary in a model
synthesised from the exact pinned OpenTitan sources. Failure to establish that
boundary is an ordinary static refusal and retains exact monolithic checking.

## Authoritative source boundary

The retained sources are byte-identical files from OpenTitan child commit
`86db2898288664d8d5e8fc635b48951ef63e3439`:

- `hw/ip/pwm/rtl/pwm_core.sv`, SHA-256
  `618998be0948d1570e7bd5fc4db6332470f02dba9b7154aa71edc8929202d855`;
- `hw/ip/pwm/rtl/pwm_chan.sv`, SHA-256
  `0b6a8cac19d1e8ae4b04ab63fd146a105b85e2ce690084beaa24aa950faca68a`;
- `hw/ip/pwm/rtl/pwm_reg_pkg.sv`, whose digest must be frozen when retained;
- the existing exact Apache-2.0 licence and mail-formatted upstream patch; and
- pinned Yosys commit `b8e7da6f40ae8f552c116bf6c359b07c6533e159`.

An authored top-level harness may drive the generated register structure and
declare properties. It may not copy, simplify, or replace the selected core or
channel transition equations. Counter-width specialisation is allowed only
through existing module parameters and must preserve truncation, overflow,
reset, duty-cycle, blink, heartbeat, phase-delay, wrap, enable, inversion, and
registered-output semantics.

## Extraction hypothesis

After hierarchy-preserving synthesis, GCC can identify each `u_chan` instance,
derive its complete state and combinational support, and construct:

1. a shared-core region containing shared state and configuration drivers;
2. one complete local region per channel instance;
3. an ordered boundary map for every edge entering or leaving each region;
4. an injective state-renaming map; and
5. independently checkable evidence that no omitted edge or hidden shared state
   crosses the declared boundary.

The independent checker receives the source-attested monolithic model and the
candidate partition. It recomputes transitive support, state ownership, edge
cuts, widths, and instance coverage. It must not trust producer-supplied region
membership or symmetry labels.

## Frozen configurations

The source is built at 2, 4, and 6 channels. Every size retains:

- independent per-channel enable and inversion writes;
- independent phase delay, blink mode, heartbeat mode, duty-cycle A/B, and
  blink X/Y values;
- shared beat and phase counters;
- per-channel captured enable and inversion state;
- all channel blink, heartbeat, duty-cycle, wrap, and registered-output state;
  and
- at least two equal-configuration channels and one distinct-configuration
  channel when the channel count permits it.

The equal-configuration pair is not assumed equivalent merely because register
values match. Reuse requires the checker to prove equal ordered boundary
signals for the complete bounded horizon. Distinct or unproved boundaries use
ordinary exact evidence.

## Predeclared properties

Before measurement, the harness freezes stable property identifiers for:

- reset-low output safety for every channel;
- local enable and inversion update effects;
- unrelated-channel noninterference;
- blink and heartbeat progression;
- phase-delay and wrap behaviour;
- simultaneous first-and-last-channel updates;
- at least one SAFE and one UNSAFE bounded control; and
- one trace whose support crosses the shared core and two channel regions.

Every family answer and earliest bad frame must equal the source-attested
monolithic model.

## Admission and hostile controls

Extraction refuses before proof reuse for any of the following:

- missing or duplicate hierarchy paths;
- incomplete node, state, or edge coverage;
- overlapping local state ownership;
- a channel-to-channel edge not represented through the shared boundary;
- mismatched source, synthesis recipe, parameter, or monolithic-model digest;
- unequal or unproved boundary signals in a claimed orbit;
- altered instance order, port width, state map, property, or horizon;
- resource excess during support reconstruction or equality checking; or
- malformed, truncated, trailing, or non-canonical evidence.

A deliberately injected hidden dependency from channel zero to channel one
must refuse reuse. Malformed evidence, verifier disagreement, and resource
exhaustion fail closed and are never converted into exact fallback. A valid
but statically unsupported partition may take the explicit source-bound exact
route.

## Baselines and measurements

The comparison includes:

- GCC source-attested monolithic exact evidence;
- GCC extracted mixed-orbit evidence, including singleton exact members;
- pinned Yosys plus rIC3 and Certifaiger whole-model evidence; and
- sequential and fixed four-way maintained orchestration.

Five clean trials at each channel count retain synthesis, production, checking,
wall time, peak RSS scope, checker work, complete artifact bytes, answers, bad
frames, and deterministic digests. No row is discarded selectively. Tiny
in-process timings cannot satisfy the product gate.

## Decision rule

The extraction mechanism passes only if exact upstream sources synthesize
reproducibly, the independent checker proves complete region boundaries, every
answer and bad frame agrees, the hidden-coupling control refuses reuse, and
mixed-orbit evidence grows more slowly than duplicated exact evidence.

Passing is evidence for a source-attested product integration, not algorithmic
novelty. Symmetry reduction, cone decomposition, and compositional model
checking are established prior art. A separate prior-art audit and a genuinely
distinct invariant would still be required before any novelty claim.
