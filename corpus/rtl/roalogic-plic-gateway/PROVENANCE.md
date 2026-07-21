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

`wrapper-predicate-set.sv` is GCC-authored Apache-2.0 verification
instrumentation. It fixes `MAX_PENDING_COUNT=3`, exposes the five independent
control inputs, and adds two bounded protocol-integrity requirements. It does
not modify the upstream module and must not be described as an upstream proof,
requirement, or product guarantee.

The canonical builder requires Yosys commit
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`. It replaces only the generated
Yosys identification comment with a revision-bound canonical comment. Semantic
BTOR2 lines are unchanged.

