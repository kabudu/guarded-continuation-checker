# OpenTitan dual-timer composed-witness baseline v1

Status: validated locally on arm64. Hosted Linux reproduction and independent
implementation review remain open.

## Question

Can predicate-set v3's OpenTitan result outperform the closest maintained
proof-carrying hardware route only because that route was tested at a different
scope?

This experiment translates the same pinned dual-timer wrapper into bounded
AIGER models for horizons 4, 5, 7, and 9. It keeps wake, bark, and bite as
separately checkable properties, preserves the timer counts and bound counter
as common observable state, and compares all twelve answers with pinned rIC3,
Certifaiger 10.2.0 plus `lrat_isa`, and `aigsim`. SAFE witness circuits are
composed for the three-property horizon-4 set and the two-property horizon-5
set using the repository's reviewed FM 2026 Theorem 1 baseline.

The bounded instrumentation uses an autonomous saturating frame counter. A bad
property is active through the selected frame and permanently disabled after
it. The OpenTitan reset remains the same unconstrained semantic input. Reset
can delay a violation but cannot create an earlier one. The output count and
frame signals retain identical latch transition definitions across
property-specific models, which is required for faithful witness composition.

## Retained result

All twelve external answers agree with GCC:

| Horizon | Wake | Bark | Bite |
| ---: | --- | --- | --- |
| 4 | SAFE | SAFE | SAFE |
| 5 | SAFE | UNSAFE at 5 | SAFE |
| 7 | UNSAFE at 7 | UNSAFE at 5 | SAFE |
| 9 | UNSAFE at 7 | UNSAFE at 5 | UNSAFE at 9 |

Certifaiger accepts all six SAFE certificates with every generated SAT proof
checked by `lrat_isa`. `aigsim` replays all six UNSAFE traces, and each trace
has exactly the expected number of frame valuations. Two clean rIC3 runs
produce byte-identical evidence for every row. Two clean Yosys builds produce
byte-identical AIGER models and witness maps.

At horizon 4, the three independent SAFE witnesses total 61,726 bytes and the
verified composed witness is 26,984 bytes, a 56.29% reduction. At horizon 5,
the two SAFE witnesses total 41,098 bytes and the verified composition is
24,292 bytes, a 40.89% reduction. The corresponding shared models are 30,695
and 30,470 bytes. GCC's source-bound artifacts are 445 and 454 bytes, but they
encode recurrence claims checked by GCC rather than general AIGER witness
circuits. The size difference is a useful representation trade-off, not proof
of a new algorithm or an equal trust base.

Retained data:

- [`opentitan-dual-timer-composed-witness-v1.csv`](../results/opentitan-dual-timer-composed-witness-v1.csv)
- [`opentitan-dual-timer-composed-witness-v1.manifest.txt`](../results/opentitan-dual-timer-composed-witness-v1.manifest.txt)

## Reproduction

First qualify the pinned rIC3 and Certifaiger toolchains using the existing
qualification scripts. Then run:

```sh
scripts/benchmark-opentitan-dual-timer-composed-witness-v1.sh \
  target/release/guarded-continuation-checker \
  "$(command -v yosys)" \
  /tmp/ric3-output \
  /tmp/certifaiger-output \
  /tmp/opentitan-dual-timer-composed.csv \
  /tmp/opentitan-dual-timer-composed.manifest.txt
```

The harness refuses overwrites, uses no-network checker containers, validates
the exact pinned toolchain lock, checks deterministic regeneration, verifies
each evidence object independently, and checks both composed witnesses against
their complete shared models. Six hostile controls reject malformed and
truncated SAFE evidence, a SAFE witness bound to the wrong horizon, a composed
witness bound to the wrong shared model, a truncated UNSAFE trace, and a trace
replayed against a SAFE horizon.

## Conclusion and remaining gates

The identical-scope result removes external-answer disagreement as an
explanation for GCC's compact artifacts. It also confirms that established
witness composition already shares substantial evidence across the same SAFE
property sets. Predicate-set v3 remains valuable as a compact bounded
word-level product contract, but this experiment supplies no support for an
algorithmic novelty claim.

Hosted amd64 reproduction, resource measurements, and independent expert review
remain required before the baseline can close its production gate. The builder
now attests the pinned OpenTitan source and Yosys revision, while the corpus
manifest binds the wrapper and compatibility files.
