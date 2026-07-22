# OpenTitan AON watchdog experiment v1

Status: retained public-design mechanism evidence. Not production acceptance or
a novelty claim.

## Question

Can GCC consume standard Yosys BTOR2 from a production-tagged embedded RTL core,
prove a large bounded SAFE region without enumerating it, recover the exact
boundary counterexample through fail-closed fallback, and reproduce all model
and certificate bytes from pinned public source?

## Predeclared boundary

The target is OpenTitan's `aon_timer_core.sv` from production tag
`Earlgrey-PROD-M6`. The unchanged upstream bytes, exact revision, digest,
licence, compatibility layer, wrapper assumptions, and pinned Yosys revision are
recorded in the [corpus provenance](../corpus/rtl/opentitan-aon-timer/PROVENANCE.md).
The experiment tests one configured watchdog path, not the whole AON timer or
OpenTitan product.

The parser change accepts standard BTOR2 `output` observations without treating
them as safety properties, accepts optional node symbols, and removes declared
inputs that cannot reach transition, constraint, or bad-property roots. The
word-region recogniser admits only exact Boolean identity wrappers emitted by
Yosys. A changed `and` to `xor` is rejected to exact explicit search.

## Result

| Case | Horizon | Answer | Route | Logical reachable states | Certificate |
|---|---:|---|---|---:|---:|
| threshold 9 | 8 | SAFE | word-region | 45 | 299 bytes |
| threshold 9 | 9 | UNSAFE at frame 9 | explicit-search | 10 | 222 bytes |
| threshold 4,000,000,000 | 1,000,000,000 | SAFE | word-region | 500,000,001,500,000,001 | 326 bytes |

The retained [acceptance CSV](../results/opentitan-aon-watchdog-acceptance-v1.csv)
is timing-independent. The workflow regenerates both BTOR2 models, compares
them byte for byte, regenerates all certificates, verifies them independently,
and rejects source substitution, an invalid output reference, a recogniser
near-neighbour, artifact overwrite, and output-path command interpretation.

The large case demonstrates that the certificate size depends on the exact
recurrence description rather than on the represented bounded state volume. It
does not show that arbitrary OpenTitan RTL has this structure.

## Closest methods and claim boundary

Yosys BTOR export, BTOR2, word-level SMT, affine recurrence acceleration,
interval reachability, and proof-carrying model checking all have prior art.
BTOR2Tools remains the syntax and witness baseline, and maintained Bitwuzla
remains the endpoint bit-vector baseline. This cycle establishes a source-bound
product integration and a useful exact compression result. It does not establish
a new verification algorithm.

The next meaningful novelty test is composition: preserve a compact proof across
multiple interacting dense predicates or separately supplied controller and
plant contracts on public embedded or robotics designs, then compare equivalent
evidence against maintained certifying systems.

## Hosted reproduction

[Hosted run 29787171907](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29787171907)
passes on exact commit `6f0c4d4`. Ubuntu rebuilds the pinned Yosys revision,
reproduces the source-bound BTOR2 models and all certificates byte for byte,
executes the five hostile controls, and passes the maintained Bitwuzla and
official BTOR2Tools baselines. The public Rust API test also passes on Ubuntu,
macOS, and Windows. The dependency audit and reproducible Linux bundle pass in
the same run.
