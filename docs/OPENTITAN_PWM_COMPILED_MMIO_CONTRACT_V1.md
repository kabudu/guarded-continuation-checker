# OpenTitan PWM compiled-MMIO contract experiment v1

Status: binary-bound certificate cycle passes locally. Static all-path proof,
the remaining hostile-control cohort, hosted portability and independent review
remain open. No novelty claim exists.

## Product question

Can GCC derive the firmware transaction contract from revision-pinned compiled
OpenTitan code, bind concrete MMIO writes to the RTL register interface, and
reuse one independently checked extraction across the authentic PWM crosstalk
revision matrix?

This experiment replaces the authored event list from the preceding mechanism.
It does not infer a favourable schedule from RTL answers.

## Frozen public source

Version 1 uses OpenTitan commit
`d88dd7e05cc3aad4dfca7020f49f2e0542fa1a88` and the exact Apache-2.0 source:

```text
sw/device/lib/dif/dif_pwm.c
```

The selected public functions are:

- `dif_pwm_configure_channel`, which writes duty-cycle, channel-parameter,
  optional blink-parameter and shared polarity registers; and
- `dif_pwm_channels_set_enabled`, which updates the shared channel-enable
  register.

The complete source bytes, licence, commit identity and SHA-256 digests must be
retained before compilation.

## Frozen firmware call sequence

One source-authored freestanding caller invokes:

```text
dif_pwm_configure_channel(channel 0, fixed pulse configuration)
dif_pwm_channels_set_enabled(channel 0, enabled)
dif_pwm_configure_channel(channel 1, fixed pulse configuration)
observe channel 0
```

The caller supplies constants only. It may provide the minimum generated
register definitions and MMIO primitives needed to compile the unmodified DIF
function bodies, but every authored compatibility definition must be labelled
and source-bound separately.

The frozen toolchain is:

```text
image=docker.io/silkeh/clang:21-bookworm
image_digest=sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3
clang=21.1.5
llvm_objdump=21.1.5
target=riscv32-unknown-elf
march=rv32imc
mabi=ilp32
language=c11
profiles=O0,O2
linking=freestanding-static
```

Both optimization profiles are frozen before the first result. They must
produce the same canonical MMIO event stream even when instruction shape and
program locations differ. Version 1 may not tune optimization or rewrite
source after observing verification answers.

## Required extraction

GCC must derive a canonical MMIO event stream from the compiled artifact, not
from the C syntax or the authored caller:

```text
event_index
program location
read or write
register offset
value or value mask
channel ownership
source function
```

The extraction accepts only statically resolved PWM-base-relative accesses.
Unknown targets, unresolved values, indirect stores, alias ambiguity,
self-modifying code, unsupported instructions or paths outside the fixed
resource envelope refuse with no contract and no RTL answer.

The event stream must show channel 0 configuration before enablement, followed
by channel 1 configuration while channel 0 remains enabled.

## Proof boundary

The candidate certificate binds:

- exact source and compatibility-boundary digests;
- exact compiler and target identity;
- complete compiled artifact digest;
- decoded control-flow and MMIO event records;
- proof that every admitted path reaches the same canonical event stream;
- the translation from register writes to RTL stimulus;
- the existing source-bound revision-impact evidence; and
- all static decoding, path, evidence and artifact limits.

The checker independently decodes the compiled artifact, reconstructs the
admitted paths and event stream, verifies its translation, then checks every
RTL counterfactual. A contract must not mask an RTL counterexample.

## Predeclared cohort

The primary cohort contains:

- Clang `-O0` and `-O2` profiles;
- the parent and child crosstalk RTL revisions;
- four old/new atom combinations;
- five existing query classes; and
- the complete 20-result matrix.

Negative firmware controls include:

- enable channel 0 before its configuration;
- reconfigure channel 0 instead of channel 1;
- disable channel 0 before observation; and
- a runtime-selected channel index whose MMIO target cannot be resolved.

Invalid or unresolved programs are contract refusals, not SAFE RTL results.

## Hostile controls

The implementation must reject:

- source, compiler, target, optimization or artifact substitution;
- changed caller constants;
- instruction, relocation, symbol or control-flow mutation;
- omitted, reordered, duplicated or trailing MMIO events;
- register-offset, width, value-mask or channel-owner drift;
- a compiled-event certificate replayed against another binary;
- stimulus mapping or RTL revision-impact substitution;
- truncation, extension and representative single-byte mutations; and
- work exceeding static bytes, instructions, branches, paths or events.

No partial artifact may be published after refusal.

## Baselines and gates

Compare the complete candidate with:

1. direct execution of the compiled artifact against an independently written
   recording MMIO implementation;
2. a maintained binary-analysis route over the identical artifact;
3. the authored four-event contract envelope; and
4. pinned Yosys, rIC3 and Certifaiger for the identical RTL matrix.

All compilation, decoding, path exploration, proof, translation, RTL checking
and verification costs remain in the comparison.

The experiment advances only if:

1. independent execution and static extraction agree on every MMIO event;
2. the compiled contract preserves all 20 RTL answers and three semantic
   change sets;
3. the old-core crosstalk failure remains observable;
4. every unresolved or invalid program refuses without an RTL answer;
5. every hostile control fails closed;
6. two clean builds and extractions are byte-identical; and
7. one extracted contract is reused across the complete RTL matrix with lower
   total checking work than rebuilding its firmware evidence per query.

There is no predeclared speed or byte threshold. Negative results remain in the
repository.

## Claim boundary

Binary lifting, abstract interpretation, symbolic execution, MMIO tracing,
proof-carrying code, hardware/software co-verification and contract-based
design have substantial prior art. Passing establishes a bounded product
capability only. Novelty remains prohibited without an invariant that survives
an equivalent-scope maintained comparison and independent expert review.

## Interim result

The first implementation uses a fail-closed Rust RV32IMC interpreter over the
flat compiled image. It supplies the compressed-instruction expansions missing
from the pinned decoder dependency, bounds image bytes, memory, instructions
and event count, and refuses unsupported instructions or out-of-range memory.
It is a narrow extraction mechanism for this experiment, not a general RISC-V
emulator.

Two clean runs of the pinned build are byte-identical. The exact compiled
artifacts have these identities:

| Profile | ELF SHA-256 | Flat image SHA-256 |
| --- | --- | --- |
| O0 | `df37a41e91610e7d0c61221b38ec346961694b098a978802a2ca0891f05ad07f` | `fb35297e84e7d66786eef3710423f4970e33a5dd4edd75f21e467dd6c5a99dcb` |
| O2 | `b8b0de989e26e790511a854a958ed09b0fa49058873fd287de9dae1ea675c028` | `02fff087aeaf9b5e62dedc89c8a0a7dbd059b830506042cda9a4a1e01c47deeb` |

O0 executes 1,928 bounded instructions and O2 executes 657. Both return zero
and recover the same 16 semantic events. The independently compiled native
recording executable also produces those exact 16 events at both optimization
levels. Program locations and containing symbols remain profile-specific and
are retained rather than normalized away.

The recovered stream passes an exact translation into the existing four-step
semantic schedule. Every single-event value mutation, truncation and extension
tested at that boundary refuses. The translated schedule therefore preserves
the existing 20-result RTL matrix and its three minimal semantic-change sets
through the already checked firmware transaction envelope.

This result does not yet satisfy the complete experiment. In particular:

- the interpreter follows the one deterministic execution induced by the fixed
  caller and recording MMIO model; it does not yet certify all paths of a
  runtime-dependent program;
- program locations are recorded, but O0 attributes event publication to the
  non-inlined `record_event` helper while O2 attributes it to the inlined
  caller;
- the predeclared runtime-selected-channel refusal, binary-bound certificate
  and maintained binary-analysis baseline are completed in the next cycle;
- the broader compiled artifact mutation matrix remains incomplete; and
- no performance, proof-size, production-readiness or novelty conclusion is
  supported by this interim gate.

## Binary-bound certificate cycle

The next cycle is frozen before implementation. It must produce one canonical
certificate per compiled profile and independently verify it against the exact
inputs. The certificate must bind:

- the complete named upstream-source set;
- the complete named compatibility-source set;
- a canonical pinned-toolchain identity;
- the flat image and full symbol-table bytes;
- the independently parsed entry, event-count and event-array locations; and
- the complete bounded execution result, including instruction count, event
  values and event-producing program locations.

The codec must be deterministic, bounded and checksummed. Verification must
recompute all set and artifact digests, parse the symbol table again, rerun the
bounded executor and compare the complete result. Digest substitution,
single-byte image and certificate mutation, truncation, extension, reordered
or duplicate source members, changed symbols and changed toolchain identity
must refuse.

The runtime-input gate is separate from ordinary concrete execution. A
runtime-selected channel caller will enter with unknown RISC-V argument
registers. If an unknown value can influence a branch, address, stored value,
jump target, event or return value, extraction must refuse without creating a
certificate or requesting an RTL answer.

Passing the certificate mechanism closes artifact identity and the predeclared
runtime-channel refusal only. The result below separately evaluates the
maintained binary-analysis gate. All-path completeness, independent review,
production readiness and novelty remain open.

## Binary-bound certificate result

The cycle passes locally for both frozen profiles. Each canonical certificate
is 488 bytes and independently binds the named upstream and compatibility
source sets, pinned toolchain identity, complete symbol table, flat image,
parsed symbol layout and full bounded execution. The exact local identities are
retained in the
[arm64 result manifest](../results/opentitan-pwm-compiled-mmio-certificate-arm64-v1.txt).

| Profile | Certificate SHA-256 | Instructions | Events |
| --- | --- | ---: | ---: |
| O0 | `fdaaa0f1be3645a6d2a4ade537361f5cf73ddf8e578207a2422f79325c0565e0` | 1,928 | 16 |
| O2 | `1b0e2424049c6b75deeba7ddfe87e65722e5bf17e22ae9e94a11efa959f6dcec` | 657 | 16 |

Two clean compilation, extraction and certification runs reproduce every
output byte. The codec rejects every representative single-byte mutation,
truncation and extension. Independent verification rejects changed upstream
or compatibility bytes, toolchain identity, image, symbol table, duplicate
members and reordered members before accepting an extraction.

The executor now tracks whether register and memory values are known. Unknown
callee-saved values may move through the stack without being treated as
constants, but an unknown branch operand, jump target, memory address, event
field, event count or return value refuses. The separately compiled
runtime-channel caller refuses at its first channel-dependent branch with no
certificate and no RTL answer. Both fixed callers still produce their original
byte-identical images and exact event streams.

The maintained comparison uses pinned angr 9.3.0 over the exact RISC-V ELFs.
Its P-Code route independently executes O0 in 267 lifted blocks and O2 in 129
lifted blocks. It recovers the same 16 events as both GCC and the native
recorder, and two clean baseline runs are byte-identical. angr currently labels
its RISC-V P-Code engine experimental and does not infer a default calling
convention for this bare-metal target, so the baseline explicitly supplies only
the entry address, return address and stack pointer.

This closes the local maintained binary-analysis comparison for the fixed
concrete caller. It does not establish all-path completeness, a certificate
size or speed advantage, production support or novelty. angr is a
well-established multi-architecture binary-analysis framework, and agreement
with it is validation evidence rather than a differentiating claim.
