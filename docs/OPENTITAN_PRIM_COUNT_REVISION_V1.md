# OpenTitan prim_count semantic revision v1

## Question

Can GCC retain independently checkable environment evidence across a real
stable-interface firmware RTL revision that changes reachable behaviour?

## Revision

OpenTitan commit `369cffc85db0e6d5a667676a6f89987b94210e70`, titled
`[prim] Update behavior of prim_count`, changes the cross-counter reset and
clear value, removes comparison-valid gating, resets the maximum on clear, and
makes error comparison continuously active. Its parent is
`34157c7afb84a7be7b1b1250d673f9fa8a3c18ce`. The module ports are unchanged.

The cohort fixes the public parameters to a two-bit cross counter with the
down-counter output selected. Its environment applies reset and holds clear,
set, and enable inactive. The observed predicate is `cnt_o == 2'b11`.

## Result

| Revision | GCC result | First bad frame | Yosys plus Z3 |
|---|---:|---:|---:|
| Parent | SAFE | none | PASSED |
| Behaviour update | UNSAFE | 0 | FAILED as expected |

GCC reuses one unchanged environment section, semantically reverifies it,
recomputes the changed counter relation over 16,384 candidate valuations, and
performs 65,536 composed-pair checks. The retained proof changes the answer
from SAFE to UNSAFE without stale-proof acceptance.

Two clean local runs produced byte-identical models, certificates, CSV, and
manifest. The retained files are:

- [`opentitan-prim-count-revision-v1.csv`](../results/opentitan-prim-count-revision-v1.csv)
- [`opentitan-prim-count-revision-arm64-v1.manifest.txt`](../results/opentitan-prim-count-revision-arm64-v1.manifest.txt)

## Claim boundary

This result passes the first functional gate that the PLIC pair falsified: the
two revisions have different reachable semantics and different bounded
answers. It does not yet establish scholarly novelty. The current fixture is a
reviewable parameter specialisation of the pinned upstream sources. A frontend
capable of compiling the verbatim package-based SystemVerilog, followed by
model equivalence against each specialisation, remains required. A maintained
closest-system comparison at the same revision-local scope is also open.
