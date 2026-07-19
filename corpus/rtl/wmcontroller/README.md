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
