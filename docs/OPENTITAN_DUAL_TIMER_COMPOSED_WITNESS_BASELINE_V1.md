# OpenTitan dual-timer composed-witness baseline v1

Status: validated locally on arm64 and in hosted amd64 Linux. Independent
implementation review remains open.

## Question

Can predicate-set v3's OpenTitan result outperform the closest maintained
proof-carrying hardware route only because that route was tested at a different
scope?

This experiment translates the same pinned dual-timer wrapper into bounded
AIGER models for horizons 4, 5, 7, and 9. It keeps wake, bark, and bite as
separately checkable properties, preserves the timer counts and bound counter
as common observable state, and compares all twelve answers with pinned rIC3,
Certifaiger 10.2.0 plus `lrat_isa`, and `aigsim`. The static producer contract
starts IC3 and depth-ordered BMC together for every property. It accepts IC3
only for a SAFE certificate and accepts an UNSAFE trace only from BMC. SAFE
witness circuits are
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
has exactly the expected number of frame valuations. Two clean producer runs
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
- [`opentitan-dual-timer-composed-witness-amd64-v1.csv`](../results/opentitan-dual-timer-composed-witness-amd64-v1.csv)
- [`opentitan-dual-timer-composed-witness-amd64-v1.manifest.txt`](../results/opentitan-dual-timer-composed-witness-amd64-v1.manifest.txt)
- [`opentitan-dual-timer-resources-amd64-v1.csv`](../results/opentitan-dual-timer-resources-amd64-v1.csv)
- [`opentitan-dual-timer-resources-amd64-v1.manifest.txt`](../results/opentitan-dual-timer-resources-amd64-v1.manifest.txt)
- [`opentitan-dual-timer-hosted-amd64-v1.provenance.txt`](../results/opentitan-dual-timer-hosted-amd64-v1.provenance.txt)

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

The engine race is static and answer-independent, not per-formula calibration.
Both engines start before the answer is known. An IC3 UNSAT result is accepted
immediately; an IC3 SAT result remains provisional while BMC explores depths in
order. Only BMC SAT may produce the final UNSAFE trace. The harness rejects any
trace whose terminated valuation count differs from the independently frozen
earliest frame. The hosted Linux run caught this distinction when IC3 produced
a valid horizon-9 bark trace ending at frame 9 although the first bad frame is
5.

## Conclusion and remaining gates

The identical-scope result removes external-answer disagreement as an
explanation for GCC's compact artifacts. It also confirms that established
witness composition already shares substantial evidence across the same SAFE
property sets. Predicate-set v3 remains valuable as a compact bounded
word-level product contract, but this experiment supplies no support for an
algorithmic novelty claim.

Hosted amd64 reproduction and resource measurements are complete. Independent
expert review remains required before the broader production gate can close.
The builder attests the pinned OpenTitan source and Yosys revision, while the
corpus manifest binds the wrapper and compatibility files.

Hosted run 29798977299 reproduced the complete corrected twelve-row baseline
on amd64, including independent verification, both compositions, deterministic
regeneration, and all six hostile controls. The job then failed before artifact
upload because the resource harness ran the Ubuntu 24.04-built GCC composer in
the older Debian Bookworm producer container. The harness now runs the combined
producer in its declared Ubuntu 24.04 runtime container as the host user. A
clean hosted rerun and retained artifact were therefore required.

## Predeclared resource comparison

The hosted amd64 follow-up measures horizons 4 and 5 with three trials. Each
sample runs ten sequential complete invocations under pinned `runlim`, reports
wall time normalised per invocation, and retains the process-group peak RSS.
The repeated workload prevents a short GCC invocation from finishing between
memory samples.

For GCC, production creates one complete predicate-set artifact and consumption
verifies it from the BTOR2 source. For the external route, production runs the
same static IC3/BMC race for every property, accepts IC3 SAFE certificates and
BMC UNSAFE traces, then composes the SAFE members; consumption checks the
composition with Certifaiger plus `lrat_isa` and replays any UNSAFE trace with
`aigsim`. Evidence bytes therefore include the composed SAFE witness and every
required UNSAFE trace. Model bytes, producer and consumer executable footprints,
wall time, and peak RSS remain separate columns.
The external producer footprint includes both rIC3 and the GCC binary that
hosts the paper-derived composer; omitting that composer would understate the
actual experimental toolchain. The consumer footprint is the complete
qualified Certifaiger tool tree used by the checker and trace replay.

Pinned Yosys source-to-model generation is common setup and is explicitly
excluded from both timed regions. Its models and provenance remain included in
the byte accounting. The comparison is reported only after the hosted run
completes with nonzero resource measurements and exact answer agreement.

## Hosted amd64 resource result

[Hosted run 29800096071](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29800096071)
completed three trials with ten sequential full
invocations per sample. Every row has exact answer agreement, deterministic
evidence, and nonzero wall-time and peak-memory measurements.

At horizon 4, median GCC production is 0.005 seconds at 8 MB peak RSS, versus
0.860 seconds and 359 MB for the external route. Median GCC consumption is
0.003 seconds at 7 MB, versus 0.242 seconds and 143 MB. GCC retains 445 bytes
of evidence versus 26,984 bytes for the composed AIGER witness.

At horizon 5, median GCC production is 0.005 seconds at 8 MB peak RSS, versus
0.550 seconds and 271 MB. Median GCC consumption is 0.003 seconds at 7 MB,
versus 0.253 seconds and 140 MB. GCC retains 454 bytes of evidence versus
24,430 bytes for the composed SAFE witness plus the shortest UNSAFE trace.

These measurements describe this narrow recognised recurrence only. They do
not establish general SAT or model-checking superiority. The GCC executable is
98,076,152 bytes. The external producer tool set is 106,649,824 bytes, while
the external consumer tool tree is only 11,444,673 bytes, so GCC does not win
the consumer executable-footprint comparison.

The arm64 and amd64 runs preserve identical answers, earliest frames, evidence
sizes, and composed-witness hashes. The generated AAG model text differs by 15
bytes per property across the two hosts, so the repository does not claim
cross-platform model-byte identity. The hosted provenance record binds the
retained files to the successful workflow commit and artifact digest.

## Canonical-export follow-up checkpoint

The remaining AAG byte difference is isolated to Yosys build-identification
text in the AAG comment and witness-map `gennerator` field. The builder now
replaces only those two non-semantic strings with the already-attested Yosys
commit under serialization profile `canonical-yosys-revision-v1`. Two clean
arm64 exports are byte-identical, and the complete twelve-row proof,
composition, independent-verification, determinism, and hostile-control
baseline passes unchanged. The retained amd64 artifact above predates this
canonicalisation and remains bound to its original workflow. A fresh hosted
amd64 run is still required to confirm cross-platform byte identity; no such
claim is made at this checkpoint.
