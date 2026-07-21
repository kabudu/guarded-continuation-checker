# OpenTitan AON watchdog provenance

The upstream file `upstream/aon_timer_core.sv` is an unmodified copy of:

- repository: `https://github.com/lowRISC/opentitan`
- production tag: `Earlgrey-PROD-M6`
- commit: `a78922f14a8cc20c7ee569f322a04626f2ac6127`
- path: `hw/ip/aon_timer/rtl/aon_timer_core.sv`
- SHA-256: `226ed77228b49c3d9231027410b5572ae7812bb0ed76dc6679c18ef028895d2b`
- licence: Apache License 2.0

The repository root [LICENSE](../../../LICENSE) covers redistribution under the
same licence. The `compat`, `wrapper.sv`, `wrapper-predicate-set.sv`,
`wrapper-dual-timer-predicate-set.sv`,
`wrapper-dual-timer-bounded-aiger.sv`, and `normalize-yosys.sed` files are GCC
test infrastructure, not upstream OpenTitan files.

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
Predicate-set certificate v2 changes only the proof representation: the same
generated BTOR2 source and ordered bad-property nodes bind joint SAFE, mixed,
and joint UNSAFE results to one independently reconstructed recurrence.

The dual-timer builder uses Yosys `setundef -zero -init` after asynchronous
reset lowering. This explicitly models verification after reset by assigning
zero to the core prescaler state that otherwise has no BTOR2 initialiser. The
assumption is part of the source-to-model boundary and is not applied by the
general parser or verifier. Both wrapper-held timer counts have explicit zero
initialisers in their source. The resulting target does not claim arbitrary
power-on-state behaviour.

The bounded AIGER wrapper adds a four-bit autonomous frame counter that advances
independently of the semantic reset and stops after the selected horizon. Each
property is enabled only through that horizon. Timer counts and the frame are
exposed as ordinary outputs to retain identical latch transition definitions in
the separate and shared property models. Those observability outputs do not
constrain the model or change a bad property. The AIGER builder uses the same
pinned Yosys executable and emits canonical ASCII AIGER plus witness maps from
two clean-directory reproductions.
