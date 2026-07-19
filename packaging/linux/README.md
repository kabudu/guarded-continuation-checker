# Guarded Continuation Checker Linux evaluation bundle

This archive contains the statically linked GCC evaluation executable, licence,
operating guidance, versioned API contracts, an SPDX 2.3 dependency SBOM, a
capability snapshot, deterministic build metadata, and SHA-256 checksums.

It is an evaluation-ready research prototype. It is not certified,
production-qualified, or evidence that an entire device is safe.

Before execution, verify the archive and its external provenance file with the
repository's `scripts/verify-linux-evaluation-bundle.sh` script. That verifier
does not execute the candidate binary. For a bundle created by GitHub Actions,
verify both signed GitHub attestations before running any bundled executable:

```sh
gh attestation verify guarded-continuation-checker-*.tar.gz \
  --repo kabudu/guarded-continuation-checker \
  --source-ref refs/heads/master \
  --source-digest REVIEWED_COMMIT \
  --signer-workflow \
    kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml \
  --signer-digest REVIEWED_COMMIT \
  --deny-self-hosted-runners

gh attestation verify guarded-continuation-checker-*.tar.gz \
  --repo kabudu/guarded-continuation-checker \
  --source-ref refs/heads/master \
  --source-digest REVIEWED_COMMIT \
  --signer-workflow \
    kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml \
  --signer-digest REVIEWED_COMMIT \
  --deny-self-hosted-runners \
  --predicate-type https://spdx.dev/Document/v2.3
```

The unsigned local in-toto provenance file records deterministic build inputs;
it does not replace the GitHub signature. Offline verification detects
corruption and rejects invalid ELF structure, but cannot establish who built
the archive.

Replace `REVIEWED_COMMIT` with the full source and signer commit recorded in the
candidate evidence. Do not verify only against the movable `master` ref.

Run all evaluation commands on a dedicated, ephemeral Linux worker. Follow
`OPERATIONS.md` and `ISOLATION_PROFILE_V1.md`, keep proprietary inputs outside
the unpacked bundle, and retain the exact archive digest with any assessment.
