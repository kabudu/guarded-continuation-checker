# Stateful washing-process plant

This repository-authored plant is a product-shaped sampled-process model for
the pinned public washing controller. It retains water level, temperature,
wash, rinse, and spin completion state. Seven controller actions drive those
transitions. Eleven sensor outputs feed the controller on the following sampled
step.

The three safety outputs detect dry heating, spinning while water intake is
active, and water intake outside fill or rinse. This is a deterministic process
model for evaluation, not a calibrated model of a particular appliance.

Regenerate its ASCII AIGER transition with maintained Yosys:

```sh
cd corpus/rtl/wmcontroller/physical-plant
yosys -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
```

The synthesis recipe maps the declared single clock to AIGER's implicit global
transition step. The remaining unused clock input cannot alter the transition.

Verify the retained bytes with `shasum -a 256 -c SHA256SUMS`.
