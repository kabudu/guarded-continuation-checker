# Changing-plant baseline family v1

This frozen family supplies the changing-plant side of the
[composed-witness baseline](../../../../docs/COMPOSED_WITNESS_BASELINE_V1.md).
It contains the retained nominal plant and three repository-authored variants:

- `sensor-stuck` reports the lid sensor closed while retaining the physical
  door state for the safety monitor;
- `actuator-delay` applies physical actions only on the plant's alternating
  actuator tick; and
- `persistent-disturbance` latches disturbances until controller fault reset.

Every plant preserves the same eight inputs, eleven controller-facing sensor
outputs, and six safety outputs. These are deterministic evaluation fixtures,
not calibrated appliance models or external evidence.

The independent bounded-transition replay in
`tests/controller_plant_bounded_aiger_api.rs` freezes all six horizon-32
answers and shortest bad frames. Each plant has two SAFE controller-output
properties. The actuator-delay variant changes the four UNSAFE shortest frames
from `4, 7, 15, 15` to `5, 11, 19, 19`; the other variants retain the nominal
frames. This prevents the family from being four byte-different copies with an
identical measured workload.

Regenerate a variant with pinned Yosys 0.67 at revision
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`:

```sh
cd corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck
yosys --no-version -Q -q -s synthesize.ys
shasum -a 256 -c SHA256SUMS
```

The family provenance manifest regenerates all four plant models, including
the existing nominal fixture. No evidence from this fixture closes the
composed-witness or novelty gate.
