# Yosys simple RTL corpus provenance

The files under `upstream/` are byte-for-byte copies from
`https://github.com/YosysHQ/yosys` at commit
`45ea2b8d6c6e94b06ff39b0117f0961ae5c16561` (2026-07-15). They are covered by
the adjacent ISC `UPSTREAM_LICENSE`. CQ-owned wrappers and configurations are
Apache-2.0 and are deliberately separate from upstream bytes.

| Upstream path | SHA-256 |
|---|---|
| `tests/simple/always01.v` | `4d246b05dc10a3fc50217eccb1dba86bc850c65641187b4533faf321033ad8d9` |
| `tests/simple/always02.v` | `606508a50ab1c4fbd1c1eece7ddb141026b0a7325f774e3c9e6057512ebec025` |
| `tests/simple/arrays01.v` | `7946bf7d3fa7a7bceec32a448056eafa8e10d2c1c1eb1a142097c308f98cebe6` |
| `tests/simple/dff_init.v` | `8730ac3b583a2e468b2792cef3c5c289549cef5bd0320984f9c2cf878120cd8e` |
| `tests/simple/retime.v` | `8eb8ce6c7234e0f97976753b9d82d06e6ceebffdcb79b80776bd7bb002108b3a` |

Verify with `shasum -a 256 upstream/*.v`; the fail-closed corpus runner also
checks these digests before synthesis.
