# BTOR2 channel property process resources v1

Status: all predeclared local gates pass on one arm64 development host. Hosted
Linux replication remains open.

## Question

What whole-process time and peak resident memory does the governed
channel-property workflow consume when producing and freshly verifying the
retained six-channel, twelve-query OpenTitan PWM artifact?

This experiment measures an execution envelope. It is not a solver race and
has no speed or memory pass threshold. Timing never participates in admission,
routing, certificate semantics, or an answer.

## Frozen method

`scripts/benchmark-btor2-channel-property-process-resources-v1.sh` runs five
trials by default, with a hard maximum of twenty. Every trial uses:

- the retained `symbolic-class-6.btor2` model;
- six `output-high` and six `output-low` queries at horizon 2;
- the exact inclusive projected-work ceiling of 6,189,840;
- one observed certification process; and
- one separate observed verification process.

BSD or GNU `/usr/bin/time` records wall time and maximum resident set size.
The script records operating system, architecture, timing backend, artifact
bytes, artifact SHA-256, internal total microseconds, and the sum of all eight
internal phases.

## Predeclared acceptance gates

The local experiment passes only when:

1. five certification and five fresh-verification rows are retained;
2. every process succeeds with twelve exact UNSAFE results;
3. every artifact is exactly 1,568 bytes and all five SHA-256 digests match;
4. every process reports one canonical phase row;
5. every phase sum is no greater than its internal total;
6. every whole-process peak RSS is positive; and
7. the script refuses invalid trial counts and an existing output path.

Passing establishes reproducible local resource evidence only. Hosted Linux
replication, independently sourced realistic properties, a maintained
equivalent-scope comparison, and external operator review remain separate
gates.

## Local result

Five trials on macOS 26.5.2 arm64 with Rust 1.97.0 pass all seven gates. Median
results are:

| Operation | Wall time | Peak RSS | Internal total | Phase sum |
|---|---:|---:|---:|---:|
| Certification | 0.12 s | 18,726,912 bytes | 123,090 us | 122,979 us |
| Fresh verification | 0.04 s | 8,044,544 bytes | 47,714 us | 47,628 us |

All ten rows report twelve UNSAFE answers and the same 1,568-byte artifact with
SHA-256
`31db59025d13872959c11783d6f1887fd98f3bac9e0234f3da7fb88ed52e3486`.
The BSD `time` wall clock has only hundredth-second resolution, so it must not
be subtracted from the microsecond internal observation to infer overhead.

The machine-readable evidence is
`results/opentitan-pwm-channel-property-process-resources-arm64-v1.csv`.
Docker-hosted ShellCheck accepts the harness, `sh -n` accepts its syntax, and
the invalid-trial and no-overwrite checks both return exit code 2.
After measurement, the harness was additionally hardened to publish through an
atomic no-overwrite hard link. An injected host-measurement failure leaves no
partial CSV, and a one-trial host rerun confirms successful atomic publication.
