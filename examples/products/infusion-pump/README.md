# Infusion-pump firmware safety gate

This product-shaped example shows CQ-SAT/GCC protecting a firmware pull request,
not merely benchmarking a solver. The fictional pump controller has two external
inputs—`motor_request` and `door_open`—and one safety requirement:

> The delivered motor enable must never be active while the medication door is
> open.

The example is intentionally small and is **not medical-device software or a
certification claim**. It demonstrates the developer workflow a real product can
adopt around a formally encoded controller.

## Run the release gate

Build the verifier, then check the protected controller:

```sh
cargo build --release
./target/release/continuation-quotient-sat \
  firmware-safety-gate \
  examples/products/infusion-pump/firmware/safe-controller.aag \
  100 target/firmware-safety/safe
```

The command proves that no declared bad state is reachable through frame 100,
writes `safety-report.txt` and `solver-metrics.csv`, and exits with status 0.

Now reproduce a firmware regression that removed the door interlock:

```sh
./target/release/continuation-quotient-sat \
  firmware-safety-gate \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  100 target/firmware-safety/regression
```

The command exits with status 1, emits a GitHub Actions error annotation, and
records the shortest failure at frame 1. Its trace shows the motor request at
frame 0 followed by the door opening while the latched motor remains active.
That trace is a concrete reproduction recipe for the firmware developer.

Exit status 2 is reserved for malformed models, invalid arguments, or verifier
failures, so CI can distinguish a rejected product build from a broken tool.

## Add it to a product repository

Copy [`.github/workflows/firmware-safety.yml`](.github/workflows/firmware-safety.yml)
into the product repository. It runs the gate on firmware pull requests and
uploads both artifacts even when a regression rejects the build.

In a production integration, the firmware build or formal-model pipeline owns
the `.aag` export. CQ-SAT/GCC consumes that declared model exactly; it does not
infer whether the model faithfully represents the source code or electronics.
