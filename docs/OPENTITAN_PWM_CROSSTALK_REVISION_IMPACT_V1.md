# OpenTitan PWM crosstalk revision impact v1

## Status

Predeclared before fixture extraction, implementation, or measurement. The
local two-atom mechanism now passes with a frozen semantic explanation. The
maintained baseline, resource comparison, hostile-drift matrix, and hosted
release-build gates remain open.

## Public revision

This cohort uses OpenTitan commit
`86db2898288664d8d5e8fc635b48951ef63e3439`, authored on 9 December 2024,
with parent `376021484b3cab4ef0d352f73d16f0b7a80c0970`:

> [pwm] Eliminate inter-channel crosstalk

The upstream commit changes two connected RTL files:

- `hw/ip/pwm/rtl/pwm_core.sv`, which changes shared-register write handling so
  an already-running channel is cleared only when its own effective enable or
  inversion state changes;
- `hw/ip/pwm/rtl/pwm_chan.sv`, which registers the channel output to prevent
  combinational glitches.

The parent and child source SHA-256 values are frozen before specialisation:

| Atom | Parent | Child |
| --- | --- | --- |
| `pwm_core.sv` | `a923f03dc4f4a89b5eb0a93c491092887168dab7ff6d814dbe653baffd2755cf` | `618998be0948d1570e7bd5fc4db6332470f02dba9b7154aa71edc8929202d855` |
| `pwm_chan.sv` | `38c0ea124dd6e933fc2152c2711f2fd2aaa6a9ee2958405ca188147e76e26e71` | `0b6a8cac19d1e8ae4b04ab63fd146a105b85e2ce690084beaa24aa950faca68a` |

The upstream licence SHA-256 is
`cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.

## Question

Can one source-bound revision-impact certificate distinguish the contribution
of both authentic connected changes, identify a property that requires their
joint application, preserve unchanged controls, and independently replay all
old/new combinations without per-formula calibration?

## Frozen atom boundary

The cohort will extract only the state and equations needed for one active PWM
channel and one unrelated shared-register write. The extracted SystemVerilog
must retain the upstream equations and cite their source lines. Width and
channel-count specialisation may remove unrelated generated register and bus
logic, but may not change the selected transition semantics.

The two change atoms are:

1. `core-clear`: old versus new `clr_chan_cntr` and captured enable/invert
   state from `pwm_core.sv`;
2. `channel-output`: old combinational versus new registered output from
   `pwm_chan.sv`.

Their ordered interface is fixed before results: the core supplies the channel
clear decision, and the channel retains its own pulse/output state. External
stimulus is shared but is not a third change atom.

## Frozen query classes

At least these five bounded query classes must be retained in one aggregate:

1. **Core-only regression:** an unrelated shared-register write must not clear
   an already-running channel. Changing only `core-clear` must change the
   result.
2. **Channel-only regression:** a same-cycle raw-output transition must not
   appear as a combinational output glitch. Changing only `channel-output`
   must change the result.
3. **Joint regression:** an unrelated shared-register write coincident with a
   raw-output transition must not disturb the externally visible output over
   the selected observation window. The query must be SAFE only when both
   atoms use the child revision, producing an inclusion-minimal invalidating
   set containing both atoms.
4. **Unchanged SAFE control:** reset must force or retain a low visible output
   under every atom combination.
5. **Unchanged UNSAFE control:** a deliberately impossible service guarantee
   must remain violated under every atom combination.

Exact horizons and output identifiers must be frozen in the retained query
manifest before the first certificate is produced. A query class is rejected
if its transition is created by wrapper-only behaviour rather than the frozen
upstream equations.

## Acceptance gates

1. Retain verbatim upstream licence and provenance, both parent/child source
   digests, the exact upstream patch, specialised sources, and derivation
   notes.
2. Pinned Yosys must generate all four old/new atom combinations from retained
   SystemVerilog without manual BTOR2 result construction.
3. The aggregate must contain all five frozen query classes and all four atom
   combinations.
4. Independent verification must replay every scenario, validate every
   embedded artifact, and rederive every minimal invalidating set.
5. Two clean productions must be byte-identical.
6. Source, atom ordering, interface, query, evidence, and result drift must fail
   closed.
7. A pinned maintained proof-producing route must agree on every scenario and
   independently check every witness or trace at identical scope.
8. Compare synthesis-inclusive producer time, independent checking time, peak
   RSS, and complete transferred bytes. Negative results remain retained.
9. The public fixture and its frozen expected certificate identity must pass
   hosted Linux release-build acceptance. Format-level three-platform identity
   remains covered by the smaller portable certificate fixture.

## Decision rule

This cohort advances the mechanism only if every integrity gate passes. It
advances a narrower research distinction only if the joint property truly
requires both authentic atoms and the closest maintained route does not already
provide an equivalent source-bound joint-change explanation at lower total
cost. The experiment is retained if it is negative.

## First local mechanism result

Pinned Yosys commit `b8e7da6f40ae8f552c116bf6c359b07c6533e159`
generates byte-identical models from two clean scratch directories. GCC creates
a byte-identical 128,768-byte aggregate with SHA-256
`e788c497b514472db64fd79fd5fa319f03abf257a3cd656c96a2eb73a44678b3`.
Independent verification replays all 20 observations across two atoms, four
combinations, and five queries.

The complete result matrix, in mask-major order, is:

| Mask | Impossible control | Core-only | Channel-only | Joint | Reset control |
| --- | --- | --- | --- | --- | --- |
| `0` old/old | UNSAFE | UNSAFE | UNSAFE | UNSAFE | SAFE |
| `1` new core | UNSAFE | SAFE | UNSAFE | UNSAFE | SAFE |
| `2` new channel | UNSAFE | UNSAFE | SAFE | UNSAFE | SAFE |
| `3` both new | UNSAFE | SAFE | SAFE | SAFE | SAFE |

The proof-carried minimal semantic-change sets are exactly:

- query 1: mask `1`, `core-clear` only;
- query 2: mask `2`, `channel-output` only;
- query 3: mask `3`, both connected atoms.

This revealed and corrected an important API ambiguity. Evidence invalidation
and semantic answer change are not the same. CLI v2 and the public Rust API now
report `minimal_invalidating_sets` for stale evidence separately from
`minimal_semantic_change_sets` for actual SAFE/UNSAFE transitions. The latter
is derived from the complete independently replayed table rather than a
heuristic attribution model.

An earlier extraction probe retained the legitimate initial clear but omitted
the channel's restart before the unrelated write. It consequently produced no
expected core or joint transition. The corrected source explicitly retains
that restart in the documented GCC scaffolding boundary; no query identifier
or horizon was changed after the failed probe.
