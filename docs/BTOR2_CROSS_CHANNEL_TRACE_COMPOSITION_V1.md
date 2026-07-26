# BTOR2 cross-channel trace composition experiment v1

Status: source-bound structural-constant mechanism, bounded self-service wire
workflow and maintained Yosys plus Z3 comparison pass locally. Mixed exact
fallback at product scale, whole-process resources, portability, compatibility
history and independent acceptance remain open.

## Product question

Can GCC express and independently check bounded temporal relationships between
two authentic RTL channels, then reuse one exact proof only when a
source-derived channel-pair equivalence class makes that reuse sound?

The preceding trace-monitor experiment handles one Boolean channel at a time.
This cycle moves the observation boundary to a same-frame relation between two
channels, then applies the existing masked finite-history semantics to that
relation.

## Frozen first relation language

Version 1 observes exactly one of:

- `Equal`: the two selected Boolean channel observations are equal; or
- `Different`: the two selected Boolean channel observations differ.

The resulting Boolean observation is consumed by the existing version-1 trace
pattern:

```text
length: 1..=8
mask:   length significant bits
value:  length significant bits, with value & !mask == 0
```

The two channel indexes must be distinct and in range. Version 1 does not
accept arithmetic comparisons, more than two channels, internal nodes,
different input environments, or inferred assumptions.

## Sound reuse invariant

The independent checker must derive the existing source-bound channel
partition. A pair query may reuse a representative proof only when both
ordered endpoints map to the same ordered pair of verified channel classes.
Singleton pair classes use direct exact evidence. Swapping endpoints is not
silently canonicalised, even though equality and difference are symmetric,
because later relation versions may not be.

For every derived UNSAFE result, the witness must replay against the concrete
target pair. Invalid structural admission, relation drift, endpoint drift,
source drift, or a refused exact backend returns no logical answer.

## First mechanism slice

The first implementation slice must:

1. construct a canonical BTOR2 Boolean relation from two authenticated channel
   observations;
2. feed that relation into the existing prefix-valid trace monitor without
   changing retained single-channel bytes or semantics;
3. reject identical endpoints and invalid indexes before solving;
4. agree with a separately constructed direct exact model for both relation
   kinds and both answer classes; and
5. retain negative results if proof reuse does not reduce complete evidence or
   checking work.

The complete product cycle additionally requires whole-process resources,
cross-platform identity, tagged compatibility and independent acceptance.
Canonical pair-query evidence, static aggregate preflight, exact fallback,
target witness replay, hostile codec tests, self-service file integration and
maintained Yosys plus Z3 comparison now pass locally.

## First retained result

The fixed six-channel OpenTitan PWM probe applies equality and difference to
three ordered pairs drawn from the independently verified class `[0, 2, 4]`.
It covers a frame-zero control, a two-frame transition, and constant-low and
constant-high two-frame patterns through horizon 2.

| Logical queries | Members | Structural members | Reused queries | Structural bytes | Member evidence | SAFE | UNSAFE |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 24 | 8 | 8 | 16 | 460 B | 8 B | 15 | 9 |

The structural admission proves that each pair has identical Boolean traces
under the same input sequence. The checker therefore evaluates the equality or
difference trace as a constant, rather than asking a solver to rediscover that
fact. Every UNSAFE target is reconstructed and replayed against the concrete
pair.

The naive exact horizon-2 route exceeded the configured UNSAT proof-step limit,
and release-mode explicit search exceeded its node-step limit. This is a
positive governed-capability result for the structural route, not a speed ratio
against a successful baseline. The
[complete retained rows](../results/opentitan-pwm-channel-pair-trace-probe-v1.md)
also record the separate two-channel complement control and the remaining
comparison gaps.

## Self-service artifact result

The fixed 12-query file cohort produces a 912-byte `GCCPTR01` version-1
artifact with SHA-256
`aac88db71d0b536a7b2c4aed4c3f37315e29a15affbdb1f260db94136e994e51`.
It contains four one-byte structural members for twelve logical queries and
reuses eight target queries. Certification and independent verification agree
on every answer and earliest bad frame.

The decoder enforces caller-supplied query, member, evidence and total-byte
limits before allocation. It rejects every truncation, every single-byte
mutation, trailing data, noncanonical ordering, source drift, query drift,
invalid endpoints and unknown enum tags. Publication is atomic, create-new and
no-clobber.

The retained query manifest is
`corpus/rtl/opentitan-pwm-channel-family/pair-trace-queries-v1.txt`. The
commands are `certify-btor2-channel-pair-traces`,
`verify-btor2-channel-pair-traces` and
`btor2-channel-pair-trace-cli-version`.

## Maintained semantic comparison

Pinned Yosys independently compiles the authentic six-channel SystemVerilog
harness to one SMT transition system. Z3 then checks every exact frame of all
twelve file-workflow queries using a separately constructed equality or
difference relation and temporal pattern.

All twelve results and earliest bad frames agree. The retained
[maintained comparison](../results/opentitan-pwm-channel-pair-trace-maintained-v1.md)
contains six SAFE and six UNSAFE rows at frames zero and one. This closes the
local independent semantic gate. It does not demonstrate novelty or a
performance advantage.

## Whole-process resource evidence

Five macOS arm64 trials retain byte-identical 912-byte artifacts for every
certification and fresh-verification process. Certification takes 0.05 to 0.06
seconds with 4,243,456 to 4,423,680 bytes peak RSS. Fresh verification takes
0.04 seconds with 4,210,688 to 4,276,224 bytes peak RSS. The
[complete rows](../results/opentitan-pwm-channel-pair-trace-process-resources-macos-arm64-v1.csv)
use `/usr/bin/time` around the public command boundary.

Five hosted Linux x86-64 trials retain the same artifact and logical-result
identity. Certification takes 0.10 to 0.25 seconds with 6,344,704 to 6,643,712
bytes peak RSS. Fresh verification takes 0.08 seconds with 6,316,032 to
6,520,832 bytes peak RSS. The
[complete Linux rows](../results/opentitan-pwm-channel-pair-trace-process-resources-linux-x86_64-v1.csv)
come from protected
[run 30184711008](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30184711008).
The portable component matrix in protected
[run 30184010209](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30184010209)
also reproduces the frozen wire identity on Ubuntu, macOS and Windows.

These are host observations, not cross-host performance guarantees.

## Claim boundary

Relational monitors, self-composition, symmetry reduction, bounded model
checking, SAT proofs, and representative proof reuse have substantial prior
art. Passing this experiment establishes a product capability, not scholarly
novelty. A novelty claim remains prohibited without a distinct invariant,
closest-implementation comparison, and independent expert review.
