# Independent AIGER safety model

`counter-overflow-4.aag` is an independently authored four-bit counter safety
model. Its output is a bad-state detector that becomes true on overflow.

Source: [`tniessen/aiger-safety-properties`](https://github.com/tniessen/aiger-safety-properties/blob/c8efd0251c0548dd46168db8410e6777c5f82b73/counter-overflow/counter-overflow-4.aag),
revision `c8efd0251c0548dd46168db8410e6777c5f82b73`.

Copyright © 2023 Tobias Nießen. Used under the MIT License reproduced in
[`LICENSE`](LICENSE).

Run the exact portfolio benchmark:

```sh
./target/release/continuation-quotient-sat \
  verify-cq-aiger examples/aiger/counter-overflow-4.aag \
  137 10 200000 results/local-aiger-counter.csv \
  results/local-aiger-counter-safety.txt
```

The importer validates the ASCII AIGER structure, converts the repeated latch
transition into exact layered CNF, fixes the declared initial latch state, and
turns the model's bad-state output into exhaustive reachability queries through
frame 137. It reports `UNSAFE` at frame 15 and writes a complete latch trace;
every SAT answer is checked against the CNF.

The current external boundary is intentionally explicit: ASCII `aag`, no primary
inputs, one to nine latches, at least one output, and deterministic latch updates.
Models outside that boundary are rejected rather than interpreted unsoundly.
