# OpenTitan prim_count semantic revision cohort

This cohort exercises an authentic stable-interface OpenTitan RTL revision. It
specialises `prim_count` to `Width=2`, `OutSelDnCnt=1`, and
`CntStyle=CrossCnt`, preserving the sequential assignments relevant to that
configuration.

The upstream revision changes reset and clear behaviour from a zero,
comparison-disabled down counter to a full-scale, always-compared down counter.
With the retained environment holding operational controls inactive and
applying reset, the old revision is SAFE for `cnt_o == 2'b11`; the new revision
is UNSAFE at frame zero.

Run the self-service acceptance gate with:

```sh
mkdir /tmp/opentitan-prim-count-v1
scripts/run-opentitan-prim-count-revision-v1.sh \
  /path/to/pinned/yosys /path/to/yosys-smtbmc \
  target/release/guarded-continuation-checker \
  /tmp/opentitan-prim-count-v1.csv \
  /tmp/opentitan-prim-count-v1.manifest.txt \
  /tmp/opentitan-prim-count-v1
```

The gate checks both GCC certificates, retained environment evidence,
deterministic regeneration, and a separate Yosys plus Z3 assertion oracle.
The companion verbatim-source gate compiles both untouched Git revisions with
pinned Slang-enabled Yosys and proves the specialised modules sequentially
equivalent. Their exact scope and provenance are recorded in `PROVENANCE.md`.
