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

## Trust-boundary v1 result

Two complete Darwin arm64 cycles pass the semantic, hostile-control, wall-time
and memory thresholds:

| Cycle | GCC median wall | Maintained median wall | GCC maximum RSS | Maintained minimum RSS |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.04 s | 3.64 s | 11,452,416 bytes | 68,042,752 bytes |
| 2 | 0.04 s | 2.81 s | 9,273,344 bytes | 68,042,752 bytes |

Both cycles recover the frozen semantic SHA-256, pass all five trials, refuse
all four GCC and ten maintained hostile cases, and report the same payloads:
110,831 bytes for GCC and 31,679 bytes for maintained regeneration. GCC's
payload is about 3.50 times larger.

Version 1 nevertheless fails gate 2. Its result manifest includes variable
timings and peak memory, so the two manifests are not byte-identical. The two
fresh angr image builds also differ in reported size by 677 bytes despite the
same Dockerfile, base-image digest and pinned package version. The semantic
summaries are byte-identical, but the complete manifest and setup description
are not. No positive trust-boundary result is admitted from v1.

## Predeclared identity v2

Before any further cycle, v2 separates two different evidence classes:

- an identity manifest containing only deterministic source digests, pinned
  tool policies, transferred-input digests and sizes, consumer executable
  digest, complete semantic digest and hostile-control counts; and
- a resource report containing observed setup and consumer measurements,
  including the variable built-image size.

Two new complete cycles must produce byte-identical identity manifests and
semantic summaries. Their resource reports must remain fully retained but are
not required to be byte-identical. Each cycle independently retains the same
five-trial twofold wall and memory gates. The Dockerfile digest, base-image
digest and angr version are identity fields; the locally assembled image byte
size is a setup observation until a reproducible published image exists.

This refinement does not change the cohort, routes, payload definitions,
trial count, semantic answer, hostile controls or performance thresholds.

## Identity v2 result

Two post-predeclaration cycles again pass every semantic, hostile and resource
threshold. Their certificates and semantic summaries are byte-identical, but
identity v2 fails because its clean GCC consumer executables differ:

- cycle 3:
  `e42806595114f252b27047db8c0830e53c607cc0f613898405c332db6dcfb888`;
- cycle 4:
  `e7545c5c20f2c2da5e05ea3b066a229532357e04e805426435f2afcde9fa81c8`.

The binaries have the same Mach-O UUID. Byte comparison localizes the drift to
target-directory-dependent build content. Compiler path-remapping flags do not
remove it. Two diagnostic clean builds using one deleted-and-recreated stable
target path are byte-identical with SHA-256
`7403908f5ea71119c2fa24236cc167f44608c44588e5bcb64ceeb6fe6593bada`.
No positive trust-boundary result is admitted from v2.

## Predeclared identity v3

Version 3 atomically reserves one fixed local target path, refuses concurrent
use, removes it during bounded cleanup and rebuilds the consumer there from
scratch. The complete executable remains retained and its SHA-256 remains an
identity field. Two new complete cycles must produce byte-identical executable
bytes, identity manifests, certificates and semantic summaries.

The fixed path affects tool construction only. It does not change consumer
code, transferred inputs, measured consumer commands, trial count, semantic
answer, hostile controls or thresholds. Resource and freshly assembled image
observations remain variable and fully retained.

## Identity v3 result

Cycles 5 and 6 pass every predeclared gate. Their consumer executables,
certificates, identity manifests and semantic summaries are byte-identical.
The executable SHA-256 is
`838a20e01092152058fc88aa1dd9c876de5a6a9082f37d1772be3e442befa8fb`,
and the certificate SHA-256 is
`90dfeff77ee5eeb32f0c578d3e9a37429bcf2b8665d79dea05983f83ab9c8cd3`.

| Cycle | GCC median wall | Maintained median wall | GCC maximum RSS | Maintained minimum RSS |
| --- | ---: | ---: | ---: | ---: |
| 5 | 0.04 s | 2.89 s | 9,388,032 bytes | 68,075,520 bytes |
| 6 | 0.04 s | 2.86 s | 11,436,032 bytes | 67,928,064 bytes |

The least favorable observed advantage is 71.50 times in median warm wall time
and 5.94 times in peak RSS. GCC transfers 110,831 bytes versus 31,679 bytes for
the maintained route, a 3.50-times payload premium.

The complete setup evidence remains material. Clean GCC consumer builds take
21.64 and 21.33 seconds and peak near 1.88 GiB. Fresh angr image setup takes
27.85 and 28.51 seconds; its observed image size still varies by 2,923 bytes.
GCC certificate production takes 10.56 and 10.94 seconds. None of these costs
is hidden inside the warm-consumer claim.

The retained evidence is under
[`results/opentitan-pwm-mmio-rtl-trust-boundary-v1`](../results/opentitan-pwm-mmio-rtl-trust-boundary-v1/README.md).
This closes the local identical-consumer trust-boundary gate for the frozen
cohort. Hosted reproduction, compatibility history and independent assessment
remain open.

## Predeclared hosted reproduction v1

Before observing any hosted trust-boundary result, the Ubuntu 24.04 x86-64
gate is frozen as follows:

1. one GitHub-hosted job builds pinned Yosys revision
   `b8e7da6f40ae8f552c116bf6c359b07c6533e159`, verifies the pinned Z3 4.16.0
   archive and uses the repository's pinned Rust 1.97.0 toolchain;
2. the job runs two complete clean cycles from the checked-out public
   repository, without consuming retained local result artifacts;
3. each cycle retains one warm-up and five fresh-process measurements for each
   route, recovers the frozen complete semantic answer, refuses all four GCC
   and ten maintained hostile changes, and independently passes the unchanged
   twofold wall-time and memory gates;
4. the two hosted cycles must produce byte-identical Linux consumer
   executables, certificates, semantic summaries and identity manifests; and
5. all setup, producer, payload and measured-process observations remain
   retained even when a gate fails.

The hosted executable is not required to match the Darwin Mach-O executable.
The gate tests clean-build identity within one hosted architecture, while the
certificate and semantic identities remain architecture-neutral.

No hosted threshold, trial count, input, tool revision, identity field,
payload definition, hostile control or expected answer may change after the
first complete hosted result is observed. A passing run would close bounded
Ubuntu platform reproduction for this comparison. It would not close
compatibility history, external independent assessment, production readiness,
novelty or general performance.

## Hosted reproduction v1 result

[Hosted run 30438790163](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30438790163)
passes every predeclared gate on Ubuntu 24.04 x86-64 at revision
`f44bc350d21a8747dbbe91b419bb61b7f86722ef`.

| Cycle | GCC median wall | Maintained median wall | GCC maximum RSS | Maintained minimum RSS |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.06 s | 4.95 s | 8,761,344 bytes | 67,084,288 bytes |
| 2 | 0.06 s | 4.96 s | 8,945,664 bytes | 67,076,096 bytes |

The least favorable hosted advantage is 82.50 times in median warm-consumer
wall time and 7.50 times in peak RSS. Both cycles reproduce the frozen semantic
and certificate identities, refuse all fourteen hostile changes and produce
byte-identical Linux executables, certificates, semantic summaries and
identity manifests.

The negative tradeoffs remain visible. GCC transfers about 3.50 times more
payload bytes, its clean consumer build takes about 41 seconds and peaks near
1.49 GB RSS, and certificate production takes about 19 seconds. Complete
retained evidence is under
[`results/opentitan-pwm-mmio-rtl-trust-boundary-hosted-v1`](../results/opentitan-pwm-mmio-rtl-trust-boundary-hosted-v1/README.md).

This closes bounded Ubuntu platform reproduction for the frozen comparison.
Compatibility history and external independent assessment remain open.
