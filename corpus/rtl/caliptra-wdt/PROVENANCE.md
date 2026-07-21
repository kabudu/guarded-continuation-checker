# Caliptra watchdog source provenance

The retained `upstream/wdt.sv` is an unmodified copy of the CHIPS Alliance
Caliptra RTL watchdog module.

- Repository: <https://github.com/chipsalliance/caliptra-rtl>
- Revision: `e241fc72de4024c569d89253de7bdccc6d63d809`
- Upstream path: `src/soc_ifc/rtl/wdt.sv`
- Retrieved: 2026-07-21
- SHA-256: `a9127035f25736258059b435a3d3aeba91cc93283ffcdd34bd80e2716ae84045`
- Licence: Apache-2.0, preserved in the source header

`wrapper-bounded-aiger.sv` is GCC-owned bounded instrumentation. It does not
modify the upstream module. It fixes the watchdog in two-stage cascade mode,
uses small timeout constants to expose boundary behaviour, and presents three
separately selectable safety properties to the existing AIGER comparison
toolchain.

`wrapper-predicate-set.sv` preserves the same cascade configuration without a
bounded frame counter and exposes the three properties through Yosys BTOR2.
It is GCC-owned instrumentation and does not modify the retained upstream
module.

`wrapper-word-input.sv` is GCC-owned formal instrumentation for the bounded
search v5 experiment. It connects a live two-bit timeout-period input to the
unmodified 32-bit Caliptra port, constrains zero as an illegal environment
value, and exposes the timer-one timeout assertion. Its SHA-256 is
`c3a0e6cc09a7835e5d0d05db27a6d6b2dfb526c05c5ed04bdcf8bcf13244ff30`.
