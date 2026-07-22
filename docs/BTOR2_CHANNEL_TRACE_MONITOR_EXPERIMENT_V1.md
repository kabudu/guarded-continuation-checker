# BTOR2 channel trace monitor experiment v1

Status: predeclared before implementation or measurement. The strict pattern
type, canonical monitor-model construction, in-memory proof composition, and
canonical wire artifact now pass their first semantic and hostile-boundary
tests. An eighteen-query both-answer control uses nine exact members, mixed
static solver routing, aggregate work refusal, and target witness replay. The
full retained cohort and maintained comparison now pass locally. A bounded
create-new file workflow is implemented and passes its first hostile checks.
Injected publication failure, cross-platform consumer evidence, and
whole-process resource evidence remain open.

## Product question

Can GCC extend the authenticated OpenTitan PWM channel-family workflow from
single-frame `OutputHigh` and `OutputLow` probes to useful bounded trace
properties, while retaining exact answers, source-bound representative reuse,
independent certificate checking, concrete counterexample recovery, and exact
fail-closed fallback?

This is a product-capability experiment. Finite trace monitors, symmetry
reduction, bounded model checking, and proof reuse are established techniques.
Passing does not establish a novel verification algorithm.

## Frozen predicate language

Version 1 accepts a masked forbidden pattern over one Boolean channel
observation:

```text
length: 1..=8
mask:   length significant bits
value:  length significant bits, with value & !mask == 0
```

Bit zero denotes the newest observation and bit `length - 1` denotes the
oldest. A property is violated at frame `f` only when `f + 1 >= length` and
every masked bit in the complete observation window equals `value`. Prefixes
shorter than `length` never match. Unmasked bits are ignored.

Examples include:

- `length=2, mask=0b11, value=0b01`: a low-to-high transition;
- `length=2, mask=0b11, value=0b10`: a high-to-low transition;
- `length=3, mask=0b111, value=0b010`: a one-cycle high pulse; and
- `length=3, mask=0b111, value=0b101`: a one-cycle low gap.

The bit order above is part of the public contract and must be covered by a
retained compatibility fingerprint.

## Exact composition boundary

The producer must:

1. authenticate the separately supplied property-free BTOR2 source;
2. independently admit the complete structural channel partition;
3. validate the complete ordered query set before solving;
4. append one canonical history monitor to each representative property model;
5. select the existing explicit-state or proof-carrying bitblast backend by the
   static source-derived route;
6. produce one exact member per admitted class, pattern, and horizon; and
7. publish only after every member succeeds and the complete artifact encodes
   within policy.

The verifier must reconstruct every monitor from source and query bytes,
recompute the structural admission and static route, verify every exact member,
and replay each UNSAFE valuation sequence against every target channel. It must
not trust a producer-supplied class, monitor model, route, answer, bad frame, or
target result.

SAFE representative reuse is admitted only because the structural certificate
establishes the same source-bound Boolean observation trace for every member of
the class under the same ordered semantic inputs. Version 1 cannot refer to a
second channel, internal node, firmware register, or unverified assumption.
Such a query must be rejected, not approximated.

## Versioning and compatibility

The implementation must use separate trace-query, trace-member, trace-result,
trace-artifact, and policy types. It must not add a new interpretation to the
`GCCBCP01` channel-property artifact or alter its retained 1,568-byte
fingerprint. The new binary codec must bind:

- format version and source SHA-256;
- complete structural admission bytes;
- ordered query identifiers, channel indexes, lengths, masks, values, and
  horizons;
- representative class, channel, backend, solver, and exact member bytes; and
- one envelope checksum.

Decoding must preflight outer and nested counts and byte lengths before
allocation, require canonical re-encoding, and reject trailing bytes.

The compact retained six-channel codec control encodes to exactly 4,808 bytes with
SHA-256
`bfc0cd4bed5b2b06d4bb3f08de9628c1add7cc05d6e3d54f1c76ddd12502d655`.
Independent decoding and canonical re-encoding reproduce those bytes. The
hostile test rejects every truncation, every single-byte mutation, trailing
data, source drift, ordered-query drift, an artifact one byte over its declared
limit, and evidence one byte over its aggregate limit. This fingerprint covers
the current mechanism fixture, not the still-open authentic retained cohort.

## Current gate state

The mechanism currently closes the local portions of gates 1 through 11. Gate
12 now has a create-new hard-link publication mechanism, collision preservation,
and temporary-file cleanup, but still requires an injected pre-publication I/O
failure. Gate 13 requires external
consumer jobs on Linux, macOS, and Windows. Gate 14 requires retained local and
hosted Linux whole-process evidence. No experiment pass or production claim is
made while those gates remain open.

## Predeclared cohort

The first cohort uses the retained two, four, and six-channel symbolic-class
OpenTitan PWM models. It includes:

- length-one controls equivalent to the existing high and low probes;
- low-to-high and high-to-low transition monitors;
- exact one-cycle high-pulse and one-cycle low-gap monitors; and
- at least one masked three-frame pattern with a don't-care bit.

Queries use horizons no greater than 8. Every result must be compared with a
direct exact query for the same channel and with a separately generated
maintained-tool model. The retained cohort must contain both SAFE and UNSAFE
answers; if the authentic source does not produce both, the experiment fails
rather than adding a synthetic answer case to the product result.

## Retained local cohort result

The complete 2-, 4-, and 6-channel cohort covers seven pattern shapes per
channel: both length-one controls, both two-frame transitions, `010`, `101`,
and `length=3, mask=0b101, value=0b001`. The latter ignores the middle frame.
All longer patterns use horizon 8. Each composed result agrees on answer and
earliest bad frame with a separately constructed direct per-channel exact
bitblast query.

| Model | Logical queries | Proof members | Reused queries | SAFE | UNSAFE | Artifact bytes | SHA-256 |
|---|---:|---:|---:|---:|---:|---:|---|
| symbolic-class-2 | 14 | 14 | 0 | 2 | 12 | 2,138,457 | `ae6f30429e1bb2f17aa28cd3b34d9b9d9b3f93b814d09edb43351d68e4f21541` |
| symbolic-class-4 | 28 | 21 | 7 | 4 | 24 | 4,007,729 | `f58fb29c3ca2313a831aac4a55f63d830eab66dd651cbda945536768708a0134` |
| symbolic-class-6 | 42 | 21 | 21 | 6 | 36 | 4,899,434 | `9ca8d6bdb0ee10877a29711fbb518810b28908b64b831283db5e2db3688ecf4a` |

The authentic retained cohort therefore contains 84 logical queries, 12 SAFE
and 72 UNSAFE answers, and exact representative reuse on the four- and
six-channel models. This closes the local direct-exact and both-answer gates.
Pinned Yosys plus Z3 independently agrees on all 84 answers and earliest bad
frames. The [retained maintained result](../results/opentitan-pwm-trace-maintained-v1.md)
closes the local maintained-tool gate. It also records the negative proof-size
result caused by proving that every reported bad frame is globally earliest.

## Self-service file workflow

The versioned manifest records ordered decimal `ID,CHANNEL,LENGTH,MASK,VALUE,HORIZON`
queries. The separate policy bounds query count, proof-member count, aggregate
evidence bytes, artifact bytes, and projected production work. All inputs use
bounded no-follow regular-file reads. Production authenticates and preflights
the complete batch before solving, verifies the finished artifact before
publication, and publishes through a same-directory temporary file plus an
atomic no-clobber hard link.

```console
target/release/guarded-continuation-checker \
  certify-btor2-channel-traces \
  corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2 \
  corpus/rtl/opentitan-pwm-channel-family/trace-queries-v1.txt \
  corpus/rtl/opentitan-pwm-channel-family/trace-policy-v1.txt \
  /new/result.channel-traces

target/release/guarded-continuation-checker \
  verify-btor2-channel-traces \
  corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2 \
  corpus/rtl/opentitan-pwm-channel-family/trace-queries-v1.txt \
  corpus/rtl/opentitan-pwm-channel-family/trace-policy-v1.txt \
  /new/result.channel-traces
```

`btor2-channel-trace-cli-version` reports the complete machine-readable
contract. The initial integration test covers production, independent
verification, stable per-query output, exact work refusal with no output,
query drift, artifact mutation, invalid patterns, oversized manifests,
symlinked input refusal, collision preservation, and temporary-file cleanup.
An injected write or sync failure remains required before acceptance gate 12
is fully closed.

## Maintained equivalent-scope control

A pinned maintained Yosys plus SMT solver workflow must compile the same
authenticated harness and equivalent trace monitors. The comparison gate
requires agreement for every ordered query on:

- SAFE or UNSAFE;
- earliest bad frame for every UNSAFE query; and
- the exact source revision, channel count, pattern, and horizon.

Performance has no pass threshold. Producer time, fresh-check time, peak RSS,
model bytes, evidence bytes, and process topology must still be reported so a
shared GCC workflow is not misleadingly compared with isolated maintained
processes.

## Acceptance gates

The experiment passes only if all of the following hold:

1. length-one trace monitors agree with the retained v1 high and low results;
2. all composed results agree with direct exact checking;
3. all results agree with the maintained equivalent-scope control;
4. every UNSAFE result replays on every target and has the same earliest frame;
5. both answer classes occur in the authentic retained cohort;
6. the six-channel cohort uses fewer proof members than logical queries;
7. static aggregate work is bounded before any member solver starts;
8. invalid structural admission never becomes a direct fallback answer;
9. unsupported, invalid, or over-budget queries return no logical result;
10. production and fresh verification reproduce byte-identical artifacts;
11. truncation, every retained single-byte mutation, trailing data, source
    drift, query drift, member drift, forced route, and nested oversize fail;
12. no-clobber and atomic-publication behavior survives injected failure;
13. the public Rust API compiles as an external consumer on Linux, macOS, and
    Windows; and
14. whole-process resource evidence is retained locally and reproduced on
    hosted Linux.

## Negative and falsification controls

- A pattern longer than the horizon plus one must remain well-defined and SAFE
  because no complete window exists. It must not match padded history.
- Nonzero bits above `length`, `value & !mask != 0`, zero mask, zero length,
  length above eight, duplicate query identifiers, and an out-of-range channel
  must be rejected before solving.
- A changed pattern or horizon must invalidate the artifact even when the
  resulting answer happens to be unchanged.
- A singleton structural class must use direct exact evidence and must not be
  labelled representative reuse.
- Cross-channel temporal properties are outside version 1 and must not be
  encoded as a single-channel trace monitor.
- If maintained checking supplies the same capability with a smaller package
  and lower resources, that negative result must be reported without weakening
  the correctness gates.

## Claim boundary after a pass

A pass would establish a self-service, proof-carrying trace-property language
for repeated embedded RTL channels. It would not establish general temporal
logic support, unbounded proof, firmware-register predicates, cross-channel
properties, superior performance, production readiness, or scholarly novelty.
Those remain separate experiments and gates.
