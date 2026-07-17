# Public Yosys RTL compatibility corpus

This corpus tests whether CQ-SAT/GCC preserves bounded safety results while
Yosys lowers varied public RTL constructs. It is compatibility evidence, not a
performance benchmark or a claim of broad production validation.

Five files under `upstream/` are byte-for-byte copies from the pinned Yosys
revision recorded in `PROVENANCE.md`. `SHA256SUMS` makes that claim executable,
and `UPSTREAM_LICENSE` contains the applicable ISC licence. CQ-SAT/GCC owns
`wrappers.sv`, the project configs, and the manifest; upstream did not author or
endorse the safety properties.

The twelve cases cover procedural conditionals, arithmetic, dynamic memory
indexing, initialised registers, and retiming-oriented pipelines. Every source
has one property expected SAFE and one expected UNSAFE, for a balanced total of
six of each. The unsafe cases must also recover a valid bounded counterexample.

Run the corpus with:

```sh
scripts/run-rtl-corpus.sh \
  target/release/continuation-quotient-sat \
  corpus/rtl/yosys-simple target/rtl-corpus /path/to/sby.py
```

The runner verifies source digests, parses the strict `manifest.tsv`, runs the
CQ project gate, validates each evidence bundle, and compares the result with a
dynamically generated SymbiYosys/Z3 oracle. CI additionally repeats CQ against a
digest-pinned historical Yosys 0.36 container.

Reference results live in `results/rtl-public-corpus-yosys-067-v1.csv` and
`results/rtl-public-corpus-yosys-036-v1.csv`. Tool version strings are part of
each row; timing is deliberately excluded because this corpus tests semantic
compatibility rather than speed.
