# Provenance

- Repository: `https://github.com/lowRISC/opentitan`
- Commit: `86db2898288664d8d5e8fc635b48951ef63e3439`
- Upstream licence: Apache-2.0
- Yosys commit: `b8e7da6f40ae8f552c116bf6c359b07c6533e159`
- Core parameter specialisation: `PhaseCntDw=4`, `BeatCntDw=3`
- Channel counts: 2, 4, and 6
- Symbolic class inputs: two-bit enable, inversion, and write buses

The three upstream files were retrieved from their authoritative raw GitHub
paths and are checked against the frozen source digests. The package digest was
not previously recorded in the crosstalk cohort and is frozen here.

The generated models retain 16, 26, and 36 state nodes respectively. The exact
linear increase of five states per added channel is a structural guard against
accidentally collapsing repeated channel state. State counts alone do not prove
correct region ownership; the extraction checker must establish that boundary
independently.

The separately retained symbolic-class models have 3 two-bit BTOR2 input nodes
and 17, 25, and 33 state nodes. Their model digests and structural classes are
frozen in `results/opentitan-pwm-symbolic-class-arm64-v1.csv`. The harness maps
even and odd channels to distinct caller-supplied firmware classes. Reuse is
valid only while that binding contract and the authenticated model bytes match.
