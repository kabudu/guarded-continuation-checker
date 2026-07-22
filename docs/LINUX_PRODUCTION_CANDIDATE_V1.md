# Linux production candidate v1

The first supported distribution profile is `firmware-rtl-v1`. It is a
statically linked `x86_64-unknown-linux-musl` build containing only the eight
commands listed in the production support profile. Research-only commands are
rejected before dispatch.

## Reproducibility evidence

On 22 July 2026, two clean build directories using Rust 1.97.0 on Debian
Bookworm produced byte-identical archives under an emulated Linux amd64
container:

```text
archive=guarded-continuation-checker-0.28.0-firmware-rtl-v1-x86_64-unknown-linux-musl.tar.gz
sha256=de7360ef2fdbc337818dbaa5f5d10aae0bc1edbb674eac9ced3403a28c837439
size=554133 bytes
reproducibility=PASS
offline_verification=PASS
tamper_tests=PASS
```

This local run used `GCC_ALLOW_DIRTY_BUNDLE=1` because the packaging changes
were being tested before commit. It validates the mechanism, not a releasable
artifact. A release candidate must be rebuilt from a clean commit by the hosted
workflow and pass the same checks.

## Build and verify

Build twice and exercise the complete reproducibility and tamper suite:

```sh
scripts/test-linux-production-candidate.sh /tmp/gcc-production-test
```

Build one candidate:

```sh
scripts/build-linux-production-candidate.sh dist
```

Verify its checksum, archive policy, SBOM, provenance, support profile and
embedded build information without executing the candidate binary:

```sh
scripts/verify-linux-evaluation-bundle.sh \
  dist/guarded-continuation-checker-*-firmware-rtl-v1-x86_64-unknown-linux-musl.tar.gz
```
