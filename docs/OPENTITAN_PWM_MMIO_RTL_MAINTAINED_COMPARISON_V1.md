# OpenTitan PWM MMIO-to-RTL maintained comparison v1

Status: predeclared before implementation or measurement.

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

Agreement would close the local identical-scope maintained-tool comparison
gate for this bounded cohort. It would validate GCC's result, not establish
novelty or a performance advantage. Binary symbolic execution,
hardware-software co-verification, transaction-level translation, SMT-based
RTL simulation and proof-carrying evidence are established techniques.

Novelty would still require a material mechanism distinction or advantage,
focused prior-art review and independent expert assessment. Production
promotion would still require hosted resource reproduction, compatibility
history, governed self-service interfaces and independent acceptance.
