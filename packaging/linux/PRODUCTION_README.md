# Guarded Continuation Checker Linux production candidate

This archive contains the statically linked firmware-rtl-v1 candidate,
Apache-2.0 licence, operating and isolation guidance, firmware CLI v2 and
artifact schema v4 contracts, an SPDX 2.3 dependency SBOM, deterministic build
metadata, provenance and SHA-256 checksums.

The executable supports only the commands listed in
`docs/PRODUCTION_SUPPORT_PROFILE_V1.md`. It rejects all research commands before
dispatch. This package is a release candidate until the independent security,
technical-review and design-partner gates pass. It is not a certification or a
claim that an entire device is safe.

Verify the archive, checksum, provenance and external SBOM with the bundled
`verify-bundle.sh` before execution. Offline verification does not execute the
candidate and does not authenticate its publisher. For a GitHub release, also
verify the repository's signed build and SBOM attestations against the exact
reviewed commit and workflow digest.

Run the candidate only on the supported Linux isolation profile. Follow
`docs/OPERATIONS.md`, retain the exact archive digest with every result, and
keep proprietary sources outside the unpacked distribution directory.
