# OpenTitan PWM DIF firmware provenance

This directory retains the exact public firmware source selected for the
compiled-MMIO contract experiment.

## Upstream revision

- Repository: `https://github.com/lowRISC/opentitan`
- Commit: `d88dd7e05cc3aad4dfca7020f49f2e0542fa1a88`
- Source path: `sw/device/lib/dif/dif_pwm.c`
- Source Git blob: `8079e146513b1bddebe8df9526870fd3a9acf8d1`
- Source SHA-256:
  `4c45ae57002b65701a4b32e00dcecfee20970586f718bad9602420f6430f5a97`
- Source bytes: 9,231
- Licence Git blob: `d645695673349e3947e8e5ae42332d0ac3164cd7`
- Licence SHA-256:
  `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`
- Licence bytes: 11,358

The source is Apache-2.0 and retains its original copyright and SPDX header.
The repository-level [`LICENSE`](LICENSE) is the verbatim upstream licence at
the selected commit.

## Boundary

[`upstream/dif_pwm.c`](upstream/dif_pwm.c) is verbatim and must not be edited.
Compatibility headers, generated register definitions, the freestanding
caller and MMIO recorder used by the experiment are GCC-authored scaffolding
and must remain outside the `upstream` directory.

The following commands reproduce both Git blob identifiers:

```console
git hash-object upstream/dif_pwm.c LICENSE
```

They must print, in order:

```text
8079e146513b1bddebe8df9526870fd3a9acf8d1
d645695673349e3947e8e5ae42332d0ac3164cd7
```
