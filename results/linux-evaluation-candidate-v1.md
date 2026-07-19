# Linux evaluation candidate v1 evidence

This record preserves the first signed master candidate for the reproducible
Linux evaluation bundle v1. It is distribution-path evidence, not a release,
production qualification, solver novelty result, or independent partner
acceptance.

## Bound identity

- Pull request: [#65](https://github.com/kabudu/guarded-continuation-checker/pull/65)
- Source and signer commit:
  `47aeb6990edbb9a5e6c28f871bb4891fab05af90`
- Hosted workflow:
  [run 29675023822](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29675023822)
- Workflow path: `.github/workflows/release-candidate-bundle.yml`
- Trigger: `workflow_dispatch` on `refs/heads/master`
- Runner identity: GitHub-hosted
- Archive:
  `guarded-continuation-checker-0.28.0-x86_64-unknown-linux-musl.tar.gz`
- Archive SHA-256:
  `6bb88302a8d16117d8b72dd227604c495c67ecee5e87f2669c919d1fb3701d6f`
- Rekor timestamps: 2026-07-19T06:35:52+01:00 for SLSA provenance and
  2026-07-19T06:35:53+01:00 for SPDX 2.3

The run built two clean clones in separate source and target paths, required all
four outputs to agree byte for byte, verified both, rejected corruption,
provenance substitution, symlink, traversal, overwrite, and executable-payload
fixtures, signed the archive with separate SLSA and SPDX predicates, and
retained the four output files for 14 days.

## Independent replay

The downloaded artifact passed the repository's non-executing offline verifier
inside a read-only Linux container. It reported the archive digest and source
commit above. The candidate binary was parsed as a static x86-64 ELF and was not
executed during verification.

Both GitHub attestations then passed with these mandatory policy constraints:

- repository `kabudu/guarded-continuation-checker`;
- source ref `refs/heads/master`;
- source digest and signer digest exactly equal to the recorded commit;
- signer workflow
  `kabudu/guarded-continuation-checker/.github/workflows/release-candidate-bundle.yml`;
- GitHub-hosted runner enforcement; and
- predicate type `https://slsa.dev/provenance/v1` or
  `https://spdx.dev/Document/v2.3`, respectively.

The attestations are transparency-log backed and remain verifiable after the
temporary workflow artifact expires. Evaluators must bind their own downloaded
archive to the recorded digest and exact policy rather than trusting this prose
record alone.

## Open gates

This evidence closes the first hosted Linux reproducibility and signed-candidate
milestones. It does not close the macOS distribution decision, tagged-release
compatibility, crate publication, real-design validity, independent partner
acceptance, safety qualification, or production-readiness gates.
