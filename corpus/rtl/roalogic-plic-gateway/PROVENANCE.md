# Roa Logic PLIC gateway provenance

`upstream/plic_gateway.sv` is an unmodified copy from `RoaLogic/plic` at commit
`fc6abe4ee04073539b77abfb6d53ee19b25bbc75`:

```text
upstream_path=rtl/verilog/core/plic_gateway.sv
upstream_sha256=bab7c8c1fa31b760f41bedb840288f40b61b460b82f0620f1128f622ca711a7b
licence_path=rtl/verilog/LICENSE.txt
licence_sha256=919790362f3dfbb5f921e0b31e0104e97d8ded413ea3f73238ab2d22ae1c628f
```

The retained licence requires preservation of its copyright statement and
disclaimer. `UPSTREAM_LICENSE.txt` is byte-identical to that upstream file.

The revision-local cohort also covers the earlier path revision
`e3483ddb06687799e2df81144659c3ec5eff3278`, whose source SHA-256 is
`a7f01fdf58c3bab4597b26a2c54784add31a2fa897a61bc7e59af872de284933`, and the
later path revision `2e8dc667f6ab69befaebdc30de7a9a53e925dbcc`, whose bytes
match the retained upstream file. An exact reverse patch reconstructs the
earlier source and the builder verifies both digests before synthesis. Files
under `revision-cohort/` other than that reconstructed upstream source are GCC
test infrastructure.

`wrapper-predicate-set.sv` is GCC-authored Apache-2.0 verification
instrumentation. It fixes `MAX_PENDING_COUNT=3`, exposes the five independent
control inputs, and adds two bounded protocol-integrity requirements. It does
not modify the upstream module and must not be described as an upstream proof,
requirement, or product guarantee.

`wrapper-constrained-predicate-set.sv` retains the same source and properties
and adds two GCC-authored environment assumptions. It is separate so the v3
constraint-free artifact remains reproducible byte for byte. The assumptions
are evaluation inputs, not upstream PLIC requirements or guarantees.

The canonical builder requires Yosys commit
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`. It replaces only the generated
Yosys identification comment with a revision-bound canonical comment. Semantic
BTOR2 lines are unchanged.
