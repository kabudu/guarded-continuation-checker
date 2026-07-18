# Independent AIGER safety models

`counter-overflow-4.aag` is an independently authored four-bit counter safety
model. Its output is a bad-state detector that becomes true on overflow.

Source: [`tniessen/aiger-safety-properties`](https://github.com/tniessen/aiger-safety-properties/blob/c8efd0251c0548dd46168db8410e6777c5f82b73/counter-overflow/counter-overflow-4.aag),
revision `c8efd0251c0548dd46168db8410e6777c5f82b73`.

Copyright © 2023 Tobias Nießen. Used under the MIT License reproduced in
[`LICENSE`](LICENSE).

Run the exact portfolio benchmark:

```sh
./target/release/guarded-continuation-checker \
  verify-cq-aiger examples/aiger/counter-overflow-4.aag \
  137 10 200000 results/local-aiger-counter.csv \
  results/local-aiger-counter-safety.txt
```

The importer validates the AIGER structure, converts the repeated latch
transition into exact layered CNF, fixes the declared initial latch state, and
turns the model's bad-state output into exhaustive reachability queries through
frame 137. It reports `UNSAFE` at frame 15 and writes a complete latch trace;
every SAT answer is checked against the CNF.

## Peterson mutual exclusion

`petersons-algorithm-2-threads-1-core.aag` models two threads, a nondeterministic
scheduler input, a signal input, nine latches, and the forbidden state in which
both threads occupy the critical section.

[Upstream Peterson model](https://github.com/tniessen/aiger-safety-properties/blob/c8efd0251c0548dd46168db8410e6777c5f82b73/petersons-algorithm/petersons-algorithm-2-threads-1-core.aag)

```sh
./target/release/guarded-continuation-checker \
  verify-cq-aiger \
  examples/aiger/petersons-algorithm-2-threads-1-core.aag \
  100 10 200000 results/local-aiger-peterson.csv \
  results/local-aiger-peterson-safety.txt
```

The static gate selects CDCL with `gate_reason=aiger-primary-inputs`. One
aggregate bounded-safety query proves the bad output unreachable through frame
100 for every scheduler/signal sequence and reports `SAFE`.

## SPI receiver input trace

`spi-bus-receive-e-08-bits.aag` models an eight-bit SPI receiver with three
primary inputs and 18 latches. Its output becomes true after receiving the target
bit sequence.

[Upstream SPI model](https://github.com/tniessen/aiger-safety-properties/blob/c8efd0251c0548dd46168db8410e6777c5f82b73/spi-sub-receive-e/spi-bus-receive-e-08-bits.aag)

```sh
./target/release/guarded-continuation-checker \
  verify-cq-aiger examples/aiger/spi-bus-receive-e-08-bits.aag \
  50 10 200000 results/local-aiger-spi.csv \
  results/local-aiger-spi-safety.txt
```

The verifier reports `UNSAFE`, minimizes the counterexample to frame 16, and
writes every latch and `SCLK`, `MOSI`, and `NOT_CS` input value through that
frame.

Both input-driven files come from the same upstream repository and pinned
revision as the counter model. Copyright © 2023 Tobias Nießen; the bundled
[`LICENSE`](LICENSE) applies.

The current external boundary is original five-field ASCII `aag` or binary
`aig`, at least one latch, declared outputs interpreted as bad-state detectors,
and hard variable and clause limits. Extended AIGER 1.9 sections are rejected
rather than interpreted incompletely.
