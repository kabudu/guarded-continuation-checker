# OpenTitan PWM MMIO-to-RTL maintained comparison v1

Status: all local acceptance gates pass in two clean cycles. Hosted Linux
attempt 1 fails the frozen raw-model byte-identity gate despite identical
normalized semantics.

## Question

Does GCC's proof-carrying compiled-firmware-to-RTL result agree with an
identical-scope workflow built from maintained binary-analysis, synthesis and
SMT tools, without consuming GCC's certificate, mapped RTL traces or replayed
observations?

## Frozen workload

The comparison uses the already pinned OpenTitan PWM runtime-channel caller,
both RV32IMC O0 and O2 ELFs, all 256 eight-bit channel inputs, and the exact
six-channel per-channel RTL source boundary. Each valid firmware behavior is
mapped into the frozen 17-transition base followed by exactly 16 quiescent
transitions, for 33 RTL transitions and 34 observations per member.

No horizon, input subset, compiler profile, channel class or reported metric
may change after the first maintained result is observed.

## Maintained route

The route is fixed to:

1. angr 9.3.0 executes each exact ELF with one symbolic eight-bit runtime
   channel, recovers the complete return and MMIO event behavior, and proves
   disjoint exhaustive coverage of `0..255`;
2. pinned Yosys revision
   `b8e7da6f40ae8f552c116bf6c359b07c6533e159` regenerates an SMT-LIB transition
   system from the authenticated per-channel RTL sources; and
3. Z3 4.16.0 constrains that model with independently translated MMIO
   valuations and returns the step and six-bit PWM observation at every frame.

The maintained translator is separately implemented and consumes only angr's
return and event rows. It must not call GCC's mapper, certificate decoder,
certificate verifier, BTOR2 parser or replay implementation.

## Acceptance gates

The cycle advances only if:

1. angr derives exactly six valid singleton behaviors and one invalid behavior
   covering the other 250 inputs under both compiler profiles;
2. valid behaviors have the complete canonical 16-event schedule and invalid
   behavior has the complete canonical ten-event rejecting schedule;
3. invalid inputs produce no RTL member;
4. Yosys plus Z3 reconstruct exactly six members, 198 transitions and 204
   observations for each profile;
5. maintained O0, maintained O2 and GCC have byte-identical normalized
   semantics;
6. all six maintained traces become non-zero and retain exactly two
   observation classes;
7. changed firmware, symbol, RTL source, tool identity, event order, event
   value, mapping width or continuation length refuses;
8. two clean maintained cycles reproduce every retained semantic and manifest
   byte;
9. wall time, user time, system time and maximum resident set size are reported
   separately for firmware analysis, RTL synthesis and SMT replay; and
10. every setup, translation, solving and checking cost remains visible,
    including regressions.

An unsupported tool result, timeout, ambiguous symbolic path, `unknown` SMT
answer, incomplete input partition or malformed output is a refusal. It is
never converted into a SAFE or UNSAFE result.

## Interpretation

Agreement closes the local identical-scope maintained-tool comparison gate for
this bounded cohort. It validates GCC's result, but does not establish
novelty or a performance advantage. Binary symbolic execution,
hardware-software co-verification, transaction-level translation, SMT-based
RTL simulation and proof-carrying evidence are established techniques.

Novelty would still require a material mechanism distinction or advantage,
focused prior-art review and independent expert assessment. Production
promotion would still require hosted resource reproduction, compatibility
history, governed self-service interfaces and independent acceptance.

## Local result

Two clean Darwin arm64 orchestration cycles produce byte-identical retained
manifests, firmware partitions and normalized RTL traces. Both compiler
profiles yield seven firmware behaviors: six valid singleton channels and one
invalid behavior covering inputs `6..255`. The maintained route reconstructs
six valid RTL members, zero invalid members, 198 transitions, 204 observations,
two phase-cycle classes and six non-zero traces. Every normalized observation
agrees with GCC.

The retained identities are:

- SMT-LIB SHA-256
  `286078b59f7292b7b3b65f8ac1bb4462efd0b452a37e53bafae3637192361a15`;
- normalized semantic SHA-256
  `e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`.

Eleven hostile controls change firmware magic, firmware symbol identity, RTL
source, Yosys identity, Z3 identity, event order, event value, mapping width,
phase representability, continuation length or report grammar. Every case
refuses before an RTL answer is emitted.

## Resource accounting

The two cycles retain every setup and analysis cost. The ranges below are
observations, not service limits:

| Operation | Profile | Wall seconds | Peak RSS |
| --- | --- | ---: | ---: |
| Fresh angr container, including pinned install | O0 | 18.65 to 20.64 | Docker client only |
| angr analysis inside Linux container | O0 | 0.912 to 0.957 | 189,984,768 to 190,091,264 bytes |
| Fresh angr container, including pinned install | O2 | 17.34 to 17.84 | Docker client only |
| angr analysis inside Linux container | O2 | 0.729 to 0.788 | 189,177,856 to 189,186,048 bytes |
| Shared Yosys synthesis | shared | 0.27 to 0.30 | 24,903,680 to 25,182,208 bytes |
| Maintained translation plus Z3 replay | O0 | 1.01 to 1.02 | 68,059,136 to 68,419,584 bytes |
| Maintained translation plus Z3 replay | O2 | 0.98 to 1.01 | 67,993,600 to 68,599,808 bytes |
| GCC production plus verification | O0 | 14.01 to 14.72 | 14,630,912 to 14,696,448 bytes |
| GCC production plus verification | O2 | 10.36 to 10.72 | 10,387,456 to 12,500,992 bytes |

The Docker-client RSS is deliberately not presented as angr memory. GCC's row
also includes exhaustive certificate mutation, truncation and extension
verification that the maintained row does not perform. These rows therefore
must not be used as a head-to-head performance claim.

## Hosted Linux gate

Before observing a hosted result, the Ubuntu 24.04 reproduction gate is frozen
to the same source revision, firmware images, 256-input domain, tool revisions,
33 transitions per member, observation schedule and eleven hostile controls.
The hosted workflow must:

1. build exact Yosys revision
   `b8e7da6f40ae8f552c116bf6c359b07c6533e159` from source;
2. verify the published Z3 archive digest before using Z3 4.16.0;
3. run angr 9.3.0 in the same digest-pinned Python container;
4. reproduce the retained semantic manifest byte-for-byte;
5. retain complete reports and resource rows as a downloadable workflow
   artifact;
6. report setup, in-container analysis, synthesis, SMT replay and GCC
   certificate verification separately; and
7. refuse the workflow if any semantic identity or hostile control differs.

No threshold, horizon, input partition or expected digest may change after the
first hosted result is observed. Hosted agreement will close platform
reproduction only. It will not establish production readiness, novelty or a
performance advantage.

### Hosted attempt 1

Hosted
[run 30405783137](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30405783137)
passes the complete maintained workflow, all eleven hostile controls and the
normalized semantic identity. It fails the frozen manifest comparison because
the Linux-generated SMT-LIB SHA-256 is
`f69e09d2dfbdcecf7c56771d152f0519449af7a3247dd5a7f82717a43ecab78c`,
while the Darwin-generated SHA-256 is
`286078b59f7292b7b3b65f8ac1bb4462efd0b452a37e53bafae3637192361a15`.

The raw models were not retained because the upload step was skipped after the
comparison failed. This is a workflow evidence-retention defect, not a reason
to reinterpret the result. The next run keeps the same failing criterion,
copies the synthesized SMT-LIB model into the result directory and executes
artifact upload under `always()`. The hosted gate remains open.

## Trust-boundary comparison

The maintained route answers the bounded semantic question by regenerating
and solving it. It does not emit an exchangeable proof object that a later
consumer can verify without angr, Yosys and Z3. GCC emits a bounded,
source-bound certificate that a separate verifier reconstructs from bytes.

That difference is a product capability hypothesis, not yet a novelty claim.
The next comparison must measure an identical consumer task:

- transfer and independently recheck GCC evidence; versus
- transfer sufficient maintained-route inputs and rerun the maintained
  derivation.

Tool installation, model generation, solving, checking, artifact bytes and
peak memory must all remain visible. Comparing GCC's exhaustive hostile-corpus
verification with one maintained replay is not an equivalent task and remains
prohibited.
