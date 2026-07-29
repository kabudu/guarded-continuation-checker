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
workflow defect. The next workflow revision copies the synthesized SMT-LIB
model into the result directory and retains available evidence under
`always()` without changing any comparison criterion.

No hosted gate closes from this attempt. A future portability-v2 criterion
must be predeclared only after the two generated models are retained and their
exact difference is understood.

## Attempt 2

- Run:
  [30408864467](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30408864467)
- Revision: `18a2df36005eeaa5c74eb70e7c5b74f8fab9046a`
- Platform: GitHub-hosted Ubuntu 24.04 x86-64
- Result: failed the unchanged frozen byte-identity gate

The retained raw Linux model reproduces SHA-256
`f69e09d2dfbdcecf7c56771d152f0519449af7a3247dd5a7f82717a43ecab78c`. It has
the same 1,840 lines as the Darwin model. An exact diff contains one removed
line and one added line: the first comment records the host compiler and
Yosys version suffix. After removing exactly that line, all remaining 1,839
lines are byte-identical with SHA-256
`bd0614bce84d54f935adb06ac7636ff3a3a1b67e1278ac71f3c3523024fcd03f`.
The normalized semantic SHA-256 remains
`e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`,
and all eleven hostile controls pass.

## Predeclared portability v2

Attempt 3 will remove exactly one line only when it is the first line, has the
Yosys SMT-LIB generator prefix and binds pinned revision
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`. The second line must still name
the expected top module. The raw model, raw hash and complete banner remain
retained and reported.

The hosted gate passes only if the remaining model bytes have the frozen
SHA-256 above, the normalized observations retain their complete frozen
SHA-256, all eleven hostile controls pass and GCC agrees with both compiler
profiles. No other comment, whitespace, ordering or model-body difference is
normalized.

## Attempt 3

- Run:
  [30411812266](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30411812266)
- Revision: `7c003315f8773de7929c08164ca382ec5742ccd4`
- Platform: GitHub-hosted Ubuntu 24.04 x86-64
- Result: passed the predeclared portability-v2 gate

The retained raw model again has SHA-256
`f69e09d2dfbdcecf7c56771d152f0519449af7a3247dd5a7f82717a43ecab78c`.
After the one validated generator banner is removed, its model-body SHA-256 is
the frozen
`bd0614bce84d54f935adb06ac7636ff3a3a1b67e1278ac71f3c3523024fcd03f`.
The semantic SHA-256 is the frozen
`e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`.
Both firmware compiler profiles agree with each other and GCC, and all eleven
hostile controls pass.

This closes the bounded hosted platform-reproduction gate for this maintained
OpenTitan PWM MMIO-to-RTL cohort. It does not close independent external
assessment, compatibility history, production readiness, novelty or general
performance gates.
