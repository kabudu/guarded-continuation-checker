# Guarded Continuation Checker Linux evaluation bundle

This archive contains the statically linked GCC evaluation executable, licence,
operating guidance, versioned API contracts, an SPDX 2.3 dependency SBOM, a
capability snapshot, deterministic build metadata, and SHA-256 checksums.

It is an evaluation-ready research prototype. It is not certified,
production-qualified, or evidence that an entire device is safe.

Before execution, verify the archive and its external provenance file with the
repository's `scripts/verify-linux-evaluation-bundle.sh` script. For a bundle
created by GitHub Actions, also verify its signed GitHub attestations:

```sh
gh attestation verify guarded-continuation-checker-*.tar.gz \
  --repo kabudu/guarded-continuation-checker \
  --source-ref refs/heads/master \
  --signer-workflow \
    kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml \
  --deny-self-hosted-runners

gh attestation verify guarded-continuation-checker-*.tar.gz \
  --repo kabudu/guarded-continuation-checker \
  --source-ref refs/heads/master \
  --signer-workflow \
    kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml \
  --deny-self-hosted-runners \
  --predicate-type https://spdx.dev/Document/v2.3
```

The unsigned local in-toto provenance file records deterministic build inputs;
it does not replace the GitHub signature. Offline checksum verification detects
corruption but cannot establish who built the archive.

Run all evaluation commands on a dedicated, ephemeral Linux worker. Follow
`OPERATIONS.md` and `ISOLATION_PROFILE_V1.md`, keep proprietary inputs outside
the unpacked bundle, and retain the exact archive digest with any assessment.
