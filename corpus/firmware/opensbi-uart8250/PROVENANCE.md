# OpenSBI UART 8250 firmware provenance

- Repository: `https://github.com/riscv-software-src/opensbi`
- Release: `v1.8.1`
- Source path: `lib/utils/serial/uart8250.c`
- Licence path: `COPYING.BSD`

The qualification script retrieves both pinned upstream files and refuses
unless their frozen SHA-256 identities match. It compiles the source without
editing it. Files under `compat/` are GCC-authored bounded scaffolding and are
not represented as upstream OpenSBI code.
