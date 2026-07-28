# OpenTitan PWM maintained MMIO hosted attempts v1

## Attempt 1

- Run:
  [30405783137](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30405783137)
- Revision: `c9cab77cbd0ea553e22bb9ccd349faa5b0aeff9c`
- Platform: GitHub-hosted Ubuntu 24.04 x86-64
- Result: failed the frozen byte-identity gate

The complete maintained workflow itself passed. It recovered seven firmware
behaviors, reconstructed six valid RTL members, rejected every invalid member,
matched all 204 normalized observations with GCC and refused all eleven hostile
changes. The normalized semantic SHA-256 remained
`e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`.

The raw SMT-LIB SHA-256 differed:

| Platform | SMT-LIB SHA-256 |
| --- | --- |
| Darwin arm64 | `286078b59f7292b7b3b65f8ac1bb4462efd0b452a37e53bafae3637192361a15` |
| Ubuntu x86-64 | `f69e09d2dfbdcecf7c56771d152f0519449af7a3247dd5a7f82717a43ecab78c` |

The frozen v1 gate required the complete semantic manifest, including the raw
SMT-LIB hash, to be byte-identical. The run therefore failed correctly. The
upload step did not run after the failed comparison, exposing a separate
workflow defect. The next workflow revision retains available evidence under
`always()` without changing any comparison criterion.

No hosted gate closes from this attempt. A future portability-v2 criterion
must be predeclared only after the two generated models are retained and their
exact difference is understood.
