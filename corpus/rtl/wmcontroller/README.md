# Pinned public washing-machine controller

This corpus retains one unmodified public controller source, its GPL-2.0
licence, a deterministic synthesis recipe, and the generated ASCII AIGER model.
See `PROVENANCE.md` before redistributing these files.

From this directory, verify and regenerate with:

```sh
shasum -a 256 -c SHA256SUMS
yosys -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
```

The generated transition is an experimental input. GCC rejects it from the
proof-carrying transducer backend under the frozen v1 cell and proof limits.
That rejection is a required regression, not a broken example.

The separate MTBDD backend admits the exact transition. Its two retained plant
composition answers have an independent bounded formal oracle in
`safe-monitor.sby` and `unsafe-monitor.sby`. Run both with the pinned
SymbiYosys revision documented in `PROVENANCE.md`:

```sh
scripts/test-public-washing-controller-oracle.sh /path/to/sby.py
```

The synthesis recipe sets the private `next_State` register's initial attribute
to zero before lowering, so the generated AIGER model, GCC query, and oracle all
start from the same six-bit zero state. The safe property passes through depth
32. The deliberately unsafe fill-only valve property fails at step 10, exactly
matching GCC's shortest bad frame.

The script works on a temporary copy, verifies the pinned inputs, and
regenerates the AIGER model byte for byte before running either formal job.
