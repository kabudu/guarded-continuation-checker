# Redundant sensor-voting verification

Imagine a monitoring chain where every next-frame status bit is the majority of
three overlapping sensor/status bits. Verification queries combine partial sensor
readings at different frames and ask whether a complete legal trace exists.

Run the executable example:

```sh
./target/release/continuation-quotient-sat \
  benchmark-cq-portfolio sensor-vote3 8,12 257,2049 50 10 200000 5151515 \
  results/local-sensor-vote-portfolio.csv
```

Expected behavior:

- `backend=cdcl` and `gate_reason=cdcl-fallback`;
- exact agreement and valid witnesses;
- query speed normalized to `1.0`, because the selected implementation is the
  same persistent CDCL baseline.

This example matters as much as the accelerated case: a practical portfolio must
identify when its specialization has no demonstrated advantage and decline it
without changing the answer.
