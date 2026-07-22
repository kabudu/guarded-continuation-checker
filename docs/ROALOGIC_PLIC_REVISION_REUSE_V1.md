# Roa Logic PLIC revision-local proof reuse v1

## Status

The local arm64 experiment passes. Hosted Linux and portable reproduction are
still open. This is public revision evidence for a narrow candidate mechanism,
not a novelty or production claim.

## Public revision pair

The experiment uses the only two commits that changed the upstream
`rtl/verilog/core/plic_gateway.sv` path:

- initial release: `e3483ddb06687799e2df81144659c3ec5eff3278`, source SHA-256
  `a7f01fdf58c3bab4597b26a2c54784add31a2fa897a61bc7e59af872de284933`;
- enhanced-performance revision:
  `2e8dc667f6ab69befaebdc30de7a9a53e925dbcc`, source SHA-256
  `bab7c8c1fa31b760f41bedb840288f40b61b460b82f0620f1128f622ca711a7b`.

The later revision gives reset and counter literals explicit widths. The
module interface remains unchanged. The existing pinned later source and an
exact reverse patch reconstruct the earlier bytes. The builder verifies both
digests before synthesis.

The component wrapper, monitor, properties, interface contract, build scripts,
and assertions are GCC-authored. They are not upstream requirements or
evidence that the complete PLIC is safe. `MAX_PENDING_COUNT` is fixed to three.

## Exact experiment

Pinned Yosys commit `b8e7da6f40ae8f552c116bf6c359b07c6533e159`
builds each public revision twice. Each pair must be byte-identical. The
semantic BTOR2 bodies of the two revisions are identical; their canonical
headers bind the different upstream revisions.

A separate one-bit GCC monitor retains whether `pending` was asserted in the
previous frame. It supplies two bounded properties through horizon two:

- `repeated-pending` is UNSAFE first at frame two;
- `impossible` is SAFE.

The monitor is the unchanged left component. The PLIC revision is the changed
right component. A one-bit interface contract wires the PLIC `pending` output
to the monitor input. The revision-local verifier validates the retained
monitor relation once, then verifies the changed PLIC relation, composition,
and final answer without decoding or semantically rechecking the monitor.

## Retained result

[`roalogic-plic-revision-reuse-v1.csv`](../results/roalogic-plic-revision-reuse-v1.csv)
records:

- both expected answers and the earliest UNSAFE frame across both revisions;
- a byte-identical 272-byte retained monitor relation for both properties;
- one produced changed section and one reused unchanged section during revised
  proof production;
- 4,096 complete changed-component candidate valuations;
- one decoded and semantically verified changed section per revision check;
- one reused unchanged section;
- 16,384 exact interface pair checks; and
- 448 final transition checks.

Maintained Yosys plus Z3 independently returns FAILED for repeated pending and
PASSED for the impossible property through frame two on both upstream
revisions. Source drift, interface-direction drift, certificate truncation,
and output overwrite each fail closed with exit status two and no success
output.

The retained-left file command consumes the old revision portfolio, validates
the unchanged monitor once, produces only the new PLIC section, and emits a
normal portfolio. Its observations report one produced and one reused section;
the ordinary from-scratch verifier then accepts the resulting artifact against
the new sources.

Reproduce after building the release binary and example:

```sh
cargo build --release --locked \
  --bin guarded-continuation-checker \
  --example roalogic_plic_revision_reuse
workdir=$(mktemp -d)
scripts/run-roalogic-plic-revision-reuse-v1.sh \
  /path/to/pinned/yosys /path/to/yosys-smtbmc \
  target/release/guarded-continuation-checker \
  /tmp/roalogic-plic-revision-reuse-v1.csv "$workdir"
```

## Remaining gates

This closes the first local public-revision, both-answer, exact-reuse, and
maintained semantic-control subgates. It does not yet provide:

- Linux, macOS, and Windows certificate-byte agreement;
- the full hostile matrix predeclared for revision-local proof v1;
- whole-process producer, checker, memory, and full-rebuild baselines;
- a faithful maintained composed-witness comparison; or
- evidence that the combined invariant is absent from the closest maintained
  systems.

The general parser also still requires each BTOR2 component to contain a bad
property, even when the revision-local query supplies the final property. The
component wrapper therefore contains a clearly labelled parser-enabling
assertion. Property-free component ingestion remains a product gap.
