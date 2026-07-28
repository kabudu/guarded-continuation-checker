# OpenTitan PWM per-channel RTL boundary v1

Status: source-boundary mechanism passes locally. Firmware mapping, trace
replay, evidence, maintained comparison and production gates remain open.

## Purpose

The first predicate-to-RTL binding candidate failed because its harness grouped
channels by parity. This replacement exposes every retained PWM configuration
field at a per-channel boundary so an MMIO schedule can describe one selected
channel without asserting writes to untouched siblings.

## Bound source

The boundary retains the exact OpenTitan PWM child sources at commit
`86db2898288664d8d5e8fc635b48951ef63e3439` and the authenticated interface-only
lowering already used by the channel-family corpus. Pinned Yosys revision
`b8e7da6f40ae8f552c116bf6c359b07c6533e159` produces:

- `generated/firmware-trace-6.btor2`;
- 97,036 canonical bytes;
- SHA-256
  `159adf69ab636d95195b2a65dd5d7afd46f05b3bad8326659fe07943cd425f7b`;
- 59 retained state declarations;
- 39 semantic inputs plus the clock; and
- semantic roots 48 and 74 for the step and six-channel output vectors.

All normalized configuration values are four-bit beat-domain words. The
firmware mapping must divide the exact phase-tick fields by 4,096 and reject
any non-divisible or out-of-range value. The boundary therefore makes the
fixture's reduced counter scale explicit and stays inside GCC's existing
64-bit exact word policy. The six channel-wide write, enable and inversion
vectors remain within six bits.

Every four-bit value is explicitly zero-extended within its own 16-bit
OpenTitan register lane. Dense packing across channel lanes is forbidden.

## Integrity properties

The public regression test independently parses the generated model and
requires:

- exactly 39 semantic inputs;
- separate enable, invert, parameter, duty-cycle and blink-parameter write
  domains;
- separate four-bit normalized words for every channel's phase, duty-cycle and
  blink fields;
- complete channel-wide write, enable, inversion, blink-mode and
  heartbeat-mode vectors;
- no constraint or embedded property that could silently narrow caller input;
  and
- no semantic input wider than the existing 64-bit verifier policy.

The build authenticates the pinned upstream source digests and Yosys revision,
refuses output overwrite and emits only a deterministic six-channel model.

## Claim boundary

This closes only the missing per-channel RTL source boundary. It does not prove
that firmware MMIO words are translated correctly, that a bounded RTL trace
was replayed, or that invalid firmware inputs produce no evidence. Those remain
gates of the predicate-to-RTL binding experiment.

Per-channel register harnesses and firmware-aware hardware verification are
established techniques. No novelty or production claim results.
