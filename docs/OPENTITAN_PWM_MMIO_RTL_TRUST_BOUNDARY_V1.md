# OpenTitan PWM MMIO-to-RTL trust-boundary comparison v1

## Question

For one already-produced O2 firmware revision, what must a later consumer
receive and execute to independently recover the bounded OpenTitan PWM answer?

This comparison tests two routes:

- GCC verification from certificate bytes, firmware image, symbol layout and
  the pinned BTOR2 source boundary; and
- maintained regeneration from the firmware ELF and authenticated RTL source
  using angr 9.3.0, pinned Yosys and Z3 4.16.0.

The comparison measures an identical consumer answer. It does not compare
GCC's producer or exhaustive mutation suite with one maintained replay.

## Frozen cohort and answer

The cohort is the retained O2 guarded-MMIO firmware and six-channel OpenTitan
PWM RTL composition. Both routes must independently report:

- seven firmware behaviors over all 256 runtime-channel inputs;
- six valid RTL members and zero invalid RTL members;
- 198 RTL transitions and 204 observations;
- two phase-cycle classes and six non-zero traces; and
- normalized semantic SHA-256
  `e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`.

Any disagreement, incomplete domain, malformed input, changed source identity,
unknown report row or tool-identity drift fails the cycle.

## Consumer payloads

GCC receives:

- `compiled-mmio-rtl-certificate-v1.bin`;
- `firmware.bin`;
- `firmware.symbols.txt`; and
- `firmware-trace-6.btor2`.

The maintained route receives:

- `firmware.elf`;
- the authenticated PWM source triplet;
- `firmware-trace-harness.sv`; and
- the versioned derivation policy and tool identities.

Payload bytes are reported exactly. No payload-size advantage is required.
Repository code, the consumer executable and installed maintained tools are
reported separately as tool setup rather than hidden inside either payload.

## Resource protocol

One untimed warm-up precedes five measured process trials per route. Every
trial starts a fresh consumer process and retains wall time, user time, system
time and peak resident set size. The maintained trial includes firmware
analysis, RTL synthesis, translation and SMT solving. The GCC trial includes
certificate parsing, checksum and source-identity checks, nested firmware
verification, mapping reconstruction and independent RTL replay.

Tool setup is measured separately:

- build the release GCC consumer from the pinned Rust toolchain; and
- provision the digest-pinned Python environment with angr 9.3.0.

Pinned Yosys and Z3 identities, executable sizes and container identity remain
visible. Setup measurements are descriptive because they are amortized across
consumer jobs.

## Predeclared gates

The experiment succeeds only if:

1. all ten measured trials return the complete frozen answer;
2. two clean complete cycles produce byte-identical manifests and summaries;
3. GCC accepts its clean payload and refuses certificate, firmware, symbol and
   BTOR2 source drift;
4. the maintained route accepts its clean payload and refuses firmware, RTL,
   Yosys, Z3 and report-schema drift;
5. the GCC median warm-consumer wall time is at most half the maintained
   median;
6. the largest GCC warm-consumer peak RSS is at most half the smallest
   maintained peak RSS; and
7. all setup, payload, process and answer measurements remain present.

No threshold, trial count, profile, tool version, payload definition or answer
identity may change after the first complete measured result is observed.

## Claim boundary

Passing would establish a bounded, measured deployment property: a consumer
can independently recheck the GCC result with less warm execution time and
memory than regenerating the same answer with this maintained workflow.

It would not establish universal superiority, production readiness or
novelty. The result applies only to this frozen cohort and toolchain and must
retain any setup-time or payload-size disadvantages.
