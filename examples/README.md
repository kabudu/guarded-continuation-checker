# Executable verification examples

These examples connect the research engine to repeated bounded-model-checking
workloads. They are deliberately small, stylized transition models, not certified
industrial controllers. Each command constructs exact layered CNF, issues a fixed
batch of partial observations, compares the selected backend with persistent
CDCL, and validates every returned witness.

## Product-shaped firmware CI

The [infusion-pump firmware safety gate](products/infusion-pump/README.md) wraps
Yosys and the exact verifier in a developer-facing product workflow. It starts
from SystemVerilog, provides named traces, CI exit semantics, GitHub annotations,
durable provenance artifacts, a copyable pull-request job, independent
SymbiYosys/Z3 checks, and passing and deliberately regressed controller builds.

## Watchdog and interlock trace analysis

[`watchdog-controller.md`](watchdog-controller.md) models a dense network of nine
latched watchdog/interlock signals. Repeated diagnostic questions ask whether
partial observations at different frames can belong to one valid execution. The
calibration-free gate selects CQ-SAT/GCC because the transition encoding is dense
and the declared batch contains enough queries to amortize compilation.

## Redundant sensor voting

[`redundant-sensor-monitor.md`](redundant-sensor-monitor.md) models overlapping
three-sensor majority voters. Here the specialized backend is not reliably
faster. The same gate selects persistent CDCL, demonstrating the product's exact
no-regression path rather than forcing the research backend onto every model.

## Independent AIGER safety models

[`aiger/counter-overflow-4.aag`](aiger/README.md) is a revision-pinned external
four-bit counter model. The standard-format importer turns its declared bad-state
output into exact bounded reachability queries and safely selects CDCL after
recognizing that the full-state query shape is outside CQ-SAT/GCC's advantage.

The same [AIGER workflow](aiger/README.md) verifies a nondeterministically
scheduled Peterson mutual-exclusion protocol as SAFE through frame 100 and
reconstructs the shortest input sequence driving an eight-bit SPI receiver's
output at frame 16. These revision-pinned external models exercise exact
primary-input semantics rather than the repository's built-in generators.
