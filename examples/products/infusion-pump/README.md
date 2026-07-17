# Infusion-pump firmware safety gate

This product-shaped example shows CQ-SAT/GCC protecting a firmware pull request
starting from readable SystemVerilog, not a hand-authored solver model. The
fictional pump controller has two external inputs—`motor_request` and
`door_open`—and one safety requirement:

> The delivered motor enable must never be active while the medication door is
> open.

The example is intentionally small and is **not medical-device software or a
certification claim**. It demonstrates the developer workflow a real product can
adopt around a formally encoded controller.

## Run the release gate

Install Yosys, build the verifier, then check the protected controller:

```sh
cargo build --release
./target/release/continuation-quotient-sat \
  firmware-rtl-safety-gate \
  examples/products/infusion-pump/rtl/safe-controller.sv \
  infusion_pump_controller 100 target/firmware-safety/safe
```

The command proves that no declared bad state is reachable through frame 100,
writes a complete evidence bundle, and exits with status 0. Yosys runs against a
staged source snapshot and emits a five-field ASCII `aag` model whose top-level
`bad` output is the safety property consumed by CQ-SAT/GCC.

Now reproduce a firmware regression that removed the door interlock:

```sh
./target/release/continuation-quotient-sat \
  firmware-rtl-safety-gate \
  examples/products/infusion-pump/rtl/door-interlock-regression.sv \
  infusion_pump_controller 100 target/firmware-safety/regression
```

The command exits with status 1, emits a GitHub Actions error annotation, and
records the shortest failure at frame 1. Its trace shows the motor request at
frame 0 followed by the door opening while the latched motor remains active.
That trace is a concrete reproduction recipe for the firmware developer.

```text
named_frame,requested_motor_active,motor_request,door_open
0,0,1,0
1,1,0,1
```

Each successful run publishes `source.sv`, `synthesis.ys`, `yosys.log`,
`yosys-errors.log`, `model.aag`, `signal.map`, `solver-metrics.csv`,
`safety-report.txt`, and `run-manifest.txt`. The manifest records the source,
top module, horizon, Yosys version, result, and `GITHUB_SHA` when CI supplies it.
It is published last, so its presence marks a complete run.

Exit status 2 is reserved for malformed models, invalid arguments, or verifier
failures, so CI can distinguish a rejected product build from a broken tool.

## Add it to a product repository

Copy [`.github/workflows/firmware-safety.yml`](.github/workflows/firmware-safety.yml)
into the product repository. It runs the gate on firmware pull requests and
uploads both artifacts even when a regression rejects the build.

## Independent oracle

The adjacent `.sby` files encode the same safety requirement as an immediate
SystemVerilog assertion. They are checked through SymbiYosys and Z3 in repository
CI: the protected controller must pass and the regression must fail at step 1.
This oracle uses a separate SMT encoding and solver path from CQ-SAT/GCC.

## Scale to a multi-module controller

[`rtl/multimodule-controller.sv`](rtl/multimodule-controller.sv) composes command
sequencing, saturating dose accounting, watchdog timing, sensor voting, and a
top-level pump system with four independent safety outputs. Generate its AIGER
model and benchmark repeated property checks:

```sh
cd examples/products/infusion-pump/rtl
yosys -Q -q -s synthesize-multimodule.ys
cd ../../../..
target/release/continuation-quotient-sat benchmark-aiger-query-reuse \
  examples/products/infusion-pump/rtl/multimodule-controller.aag \
  8,16,32,64 10 results/local-rtl-query-reuse.csv
```

The experiment shares one exact solver for bounded batches of two properties and
compares it with a fresh exact BMC solver for every property. The static
portfolio uses reuse only for multi-property encodings with at most 15,000
clauses. The checked-in result wins through horizon 16 and selects cold BMC at
horizons 32 and 64, where unrestricted reuse becomes slower.

The same design is split into two files under [`rtl/project`](rtl/project) to
exercise the project interface:

```sh
target/release/continuation-quotient-sat \
  firmware-rtl-project-safety-gate infusion_pump_system 100 \
  target/firmware-safety/project \
  examples/products/infusion-pump/rtl/project/pump-components.sv \
  examples/products/infusion-pump/rtl/project/pump-system.sv
```

## Deliberate boundary

The workflow accepts either one SystemVerilog source or a project of up to 64
ordered sources, a simple top-module identifier, and one or more explicit
top-level bad outputs. Modules are flattened before export. Yosys must lower the design
to the original five-field ASCII AIGER subset already validated by this project.
The source is capped at 10 MiB and synthesis at 120 seconds; the existing AIGER
and bounded-unrolling resource limits still apply afterwards.
Synthesis don't-care bits are explicitly lowered to zero, while unconstrained
top-level signals remain primary inputs. General AIGER 1.9 bad-state/constraint
sections, include directories, parameter overrides, and source-level assertion mapping are
not yet accepted. Unsupported synthesis or model shapes fail with exit status 2
rather than being approximated.

CQ-SAT/GCC verifies the Yosys-lowered model exactly; it does not infer whether the
RTL faithfully represents the compiler, electronics, mechanics, or medication
delivery system.
