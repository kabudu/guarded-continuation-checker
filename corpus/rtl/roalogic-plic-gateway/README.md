# Pinned Roa Logic PLIC gateway

This corpus retains one unmodified public PLIC gateway at the revision and
digests in `PROVENANCE.md`. The upstream licence and notice are preserved
separately from GCC's Apache-2.0 verification wrapper.

The wrapper fixes a small pending-event counter and adds two GCC-owned protocol
requirements. Its five semantic controls make this the retained public case for
BTOR2 bounded search certificate v3. The requirements do not represent upstream
assurance for the module or complete PLIC.

Reproduce the canonical BTOR2 model with pinned Yosys:

```sh
scripts/build-roalogic-plic-gateway-btor2-v1.sh \
  "$(command -v yosys)" /tmp/roalogic-plic-gateway.btor2
```

Reproduce GCC, hostile-control, deterministic-evidence, resource-refusal, and
maintained Yosys plus Z3 agreement with:

```sh
scripts/run-roalogic-plic-gateway-acceptance-v1.sh \
  target/debug/guarded-continuation-checker \
  "$(command -v yosys)" \
  "$(command -v yosys-smtbmc)" \
  /tmp/roalogic-plic-gateway-acceptance-v1.csv
```

Compare that output with
`results/roalogic-plic-gateway-acceptance-v1.csv`.
