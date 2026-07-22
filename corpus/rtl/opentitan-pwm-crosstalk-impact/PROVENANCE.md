# Provenance

The behavioural boundary is derived from the Apache-2.0 OpenTitan repository:

- repository: `https://github.com/lowRISC/opentitan`
- parent: `376021484b3cab4ef0d352f73d16f0b7a80c0970`
- child: `86db2898288664d8d5e8fc635b48951ef63e3439`
- child subject: `[pwm] Eliminate inter-channel crosstalk`
- author date: `2024-12-09T18:25:17Z`
- upstream licence SHA-256:
  `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`

Frozen full-source SHA-256 values:

| Source | Parent | Child |
| --- | --- | --- |
| `hw/ip/pwm/rtl/pwm_core.sv` | `a923f03dc4f4a89b5eb0a93c491092887168dab7ff6d814dbe653baffd2755cf` | `618998be0948d1570e7bd5fc4db6332470f02dba9b7154aa71edc8929202d855` |
| `hw/ip/pwm/rtl/pwm_chan.sv` | `38c0ea124dd6e933fc2152c2711f2fd2aaa6a9ee2958405ca188147e76e26e71` | `0b6a8cac19d1e8ae4b04ab63fd146a105b85e2ce690084beaa24aa950faca68a` |

The specialised sources retain these changed equations:

- parent core: clear on every shared `pwm_en`, `invert`, or channel-parameter
  write enable;
- child core: capture effective per-channel enable/invert state and clear only
  on a channel-local transition;
- parent channel: expose the selected pulse combinationally;
- child channel: register the selected pulse before exposing it.

The deterministic phase generator, narrowed pulse state, restart between the
legitimate initial clear and unrelated write, and five property outputs are GCC
verification scaffolding. They are not represented as upstream RTL. The cohort
is invalid if a retained transition is caused solely by that scaffolding rather
than one of the four frozen upstream equations.
