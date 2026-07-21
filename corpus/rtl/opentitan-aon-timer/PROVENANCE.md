# OpenTitan AON watchdog provenance

The upstream file `upstream/aon_timer_core.sv` is an unmodified copy of:

- repository: `https://github.com/lowRISC/opentitan`
- production tag: `Earlgrey-PROD-M6`
- commit: `a78922f14a8cc20c7ee569f322a04626f2ac6127`
- path: `hw/ip/aon_timer/rtl/aon_timer_core.sv`
- SHA-256: `226ed77228b49c3d9231027410b5572ae7812bb0ed76dc6679c18ef028895d2b`
- licence: Apache License 2.0

The repository root [LICENSE](../../../LICENSE) covers redistribution under the
same licence. The `compat`, `wrapper.sv`, `wrapper-predicate-set.sv`, and
`normalize-yosys.sed` files are GCC test infrastructure, not upstream OpenTitan
files.

The source is never edited in place. The build script checks its digest, copies
it into a private temporary directory, and applies the checked-in compatibility
normalisation there. The normalisation moves a package qualification onto the
one imported type and replaces three calls to OpenTitan's strict lifecycle
helper with equality against the exact encoded `Off` value, `4'b1010`. This is
needed by the pinned Yosys Verilog frontend and does not weaken the selected
constant lifecycle configuration.

The BTOR2 models are generated with Yosys commit
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`. The build uses `clean -purge` to
remove dead flattened register fields. It does not truncate live words.
Yosys records its host compiler in the first comment line. After checking the
executable revision and the expected header prefix, the build replaces only
that comment with a canonical revision-bound header. No semantic BTOR2 line is
changed.

`wrapper-predicate-set.sv` changes only the test boundary. It connects the
existing `wdog_intr_o` and `wdog_reset_req_o` signals to two assertions and
sets explicit bark and bite thresholds. It does not modify the pinned core.
