# Reproducible Linux evaluation bundle v1

This contract defines the first auditable binary distribution path for Guarded
Continuation Checker. It packages the evaluation-ready research prototype for
`x86_64-unknown-linux-musl`. It is not a production release or a replacement for
the repository's design, model, assumption, and bounded-result review gates.

## Canonical inputs

The builder accepts no behavioural flags. It requires:

- a clean Git worktree at a full 40-character revision;
- the revision's committed timestamp as `SOURCE_DATE_EPOCH`;
- Rust and Cargo 1.97.0;
- the locked dependency graph in `Cargo.lock`;
- `x86_64-unknown-linux-musl`; and
- GNU tar, gzip, jq, SHA-256 tools, and the musl linker on x86_64 Linux.

Canonical Rust flags strip symbols, remove the ELF build ID, disable incremental
compilation, and remap the source root to
`/usr/src/guarded-continuation-checker`. Archive members are sorted and receive
the source timestamp, numeric owner and group zero, canonical permissions, and
gzip without variable filename or timestamp fields.

The development-only `GCC_ALLOW_DIRTY_BUNDLE=1` escape hatch marks
`BUILD-INFO.json` as dirty. The normal verifier rejects such a bundle. CI and
attestation workflows never set the escape hatch.

## Bundle and sibling evidence

The archive contains:

- a static `guarded-continuation-checker` executable;
- Apache-2.0 licence and evaluation warning;
- operations and isolation guidance;
- firmware, predicate, and event-contract API contracts;
- an exact capability snapshot generated during the controlled build and bound
  by the internal manifest;
- deterministic build information with source, toolchain, lockfile, and binary
  digests;
- an SPDX 2.3 JSON dependency SBOM; and
- a SHA-256 manifest for every other archive file.

The output directory also contains:

- the archive checksum;
- a byte-identical external SPDX document for signing; and
- a deterministic in-toto Statement v1 with SLSA provenance v1 predicate that
  binds the archive, source revision, lockfile, toolchain, target, and epoch.

The local provenance statement is unsigned and cannot establish builder
identity. The manual `Attested Linux evaluation bundle` workflow is restricted
to `master` on GitHub-hosted runners. It uses GitHub's official `actions/attest`
action to sign both build provenance and the SPDX predicate through Sigstore,
then retains the four bundle files for 14 days. The workflow does not publish a
crate, create a tag, or create a GitHub release.

## Build and reproduce

On native x86_64 Linux:

```sh
sudo apt-get install musl-tools jq binutils
rustup target add x86_64-unknown-linux-musl --toolchain 1.97.0
scripts/build-linux-evaluation-bundle.sh /tmp/gcc-bundle
```

The reproducibility gate clones the clean revision into two distinct source
paths, builds both with separate target directories, compares all four output
files byte for byte, verifies both bundles, rejects archive and provenance
tampering, and confirms no-overwrite publication:

```sh
scripts/test-linux-evaluation-bundle.sh /tmp/gcc-bundle-repro
```

## Verify

Offline structural, checksum, static-link, provenance-subject, and SBOM
verification requires only the four output files and standard Linux tools:

```sh
scripts/verify-linux-evaluation-bundle.sh \
  guarded-continuation-checker-VERSION-x86_64-unknown-linux-musl.tar.gz \
  guarded-continuation-checker-VERSION-x86_64-unknown-linux-musl.tar.gz.sha256 \
  guarded-continuation-checker-VERSION-x86_64-unknown-linux-musl.intoto.jsonl \
  guarded-continuation-checker-VERSION-x86_64-unknown-linux-musl.spdx.json
```

Offline verification detects corruption and internal disagreement but does not
authenticate an unsigned local builder. It deliberately does not execute the
candidate binary. For a GitHub-built candidate, use `gh attestation verify`
before any execution, with all of:

- repository `kabudu/guarded-continuation-checker`;
- source ref `refs/heads/master`;
- signer workflow
  `kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml`;
- `--deny-self-hosted-runners`; and
- SPDX predicate type `https://spdx.dev/Document/v2.3` for the SBOM attestation.

Only after both attestations pass should the binary be executed, and then only
inside the dedicated ephemeral worker described by the isolation profile. The
capability commands may be replayed there against `CAPABILITIES.txt`.

## Claim boundary

V1 covers one static Linux target. macOS is still development-only, Windows is
unsupported, and no signed candidate has passed this new workflow until a
reviewed `master` run is retained. Reproducible packaging reduces supply-chain
and evaluator setup risk; it does not improve solver novelty, prove a firmware
model complete, qualify GCC under a safety standard, or close independent
acceptance.
