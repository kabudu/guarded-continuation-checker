# Provenance

- Upstream repository: `https://github.com/lowRISC/opentitan`
- Upstream path: `hw/ip/prim/rtl/prim_count.sv`
- Before revision: `34157c7afb84a7be7b1b1250d673f9fa8a3c18ce`
- Before source SHA-256: `a864392c228b1d4f4a6b4dc3baae21db3fb3afa2869d440a72623cfc9a061a45`
- After revision: `369cffc85db0e6d5a667676a6f89987b94210e70`
- After source SHA-256: `f7256b26530637658956353adf2fa99bc4fdfd25ffcc44474f62138d9cd7d78b`
- Upstream licence: Apache-2.0
- Upstream licence SHA-256: `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`
- Retrieval date: 2026-07-22

The two specialisations remove package types, unused duplicate-counter mode,
assertion macros, and generic array structure after fixing the parameters to
`Width=2`, `OutSelDnCnt=1`, and `CntStyle=CrossCnt`. They retain the upstream
reset, clear, set, enable, maximum, up-counter, down-counter, output, and error
assignments for that configuration. The independently generated Yosys plus Z3
oracle checks the resulting old SAFE and new UNSAFE behaviour.

This is currently a reviewable semantic specialisation, not a machine-checked
translation of the complete upstream SystemVerilog. A verbatim-source frontend
comparison remains a qualification gate before using this cohort as scholarly
novelty evidence.
