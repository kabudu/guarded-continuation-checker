# Public washing-machine controller provenance

`upstream/Controller.v` and `UPSTREAM_LICENSE` are byte-for-byte copies from
[`yasnakateb/WMController`](https://github.com/yasnakateb/WMController) at
revision `a81fadd25b07e3e415a57f997f7106f67e2fb24b`.

Upstream describes the project as a washing-machine control system and licenses
it under GPL-2.0. The upstream bytes remain separate from GCC's Apache-2.0 code.
Their inclusion does not change the licence of GCC, and downstream redistribution
must preserve the applicable upstream licence.

The repository-authored `synthesize.ys` records the exact synthesis
interpretation. In particular, `setattr` resolves upstream's uninitialized
`next_State` register by assigning its synthesis `init` attribute before process
lowering; `setundef -zero` resolves any remaining undefined combinational
values. `generated/controller.aag` was produced by
Yosys 0.67+post with git revision `b8e7da6f40ae8f552c116bf6c359b07c6533e159`.
The generated model is experimental evidence derived from the GPL-2.0 source
and is distributed with the same upstream licence notice.

Verify the source and licence bytes with `shasum -a 256 -c SHA256SUMS`. The
experiment must additionally regenerate the AIGER model and compare it before
publishing a result.

The maintained oracle reads that generated AIGER model without overriding its
latch initialisation. This makes the synthesis recipe, GCC, and SymbiYosys use
the same zero-initialised `next_State` interpretation. The oracle provides a
separate bounded model-checking path through SymbiYosys and Z3; it does not
claim to validate Yosys synthesis independently of Yosys.

The repository-authored formal monitors and SymbiYosys job files are
Apache-2.0 GCC code. CI runs them with SymbiYosys revision
`fea6e467d067b3ea84b6b5ac08cd48beb59f0d42`, maintained Yosys, and Z3. The
formal jobs read the pinned generated AIGER transition and its zero latch
initialisation directly, without overriding the model.
