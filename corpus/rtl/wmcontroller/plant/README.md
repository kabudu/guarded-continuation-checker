# Stateful washing-machine plant v1

This repository-authored Apache-2.0 fixture models six bits of stored physical
state: a two-bit water level, cycle progress, door state, imbalance, and motor
failure. Three unconstrained event inputs update the stored fault conditions.
The public washing controller drives fill, spin, fault, and water-intake action
inputs.

The six safety outputs cover:

- water intake with the door open;
- water intake when already full;
- spinning while imbalanced;
- spinning after a motor failure;
- physical actuation while the controller reports fault; and
- simultaneous fill and spin commands.

The first four are expected-UNSAFE reachability checks under nondeterministic
disturbances. The final two are expected-SAFE controller-output exclusions.
This is a representative mechanism model, not a calibrated physical appliance
model and not independent product evidence.

Regenerate and verify from this directory:

```sh
shasum -a 256 -c SHA256SUMS
yosys -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
```

The synthesis recipe replaces the single declared clock with AIGER's implicit
global transition step. The clock remains as an unused primary input. GCC
explores it but it cannot change sensors, next state, or safety outputs.
