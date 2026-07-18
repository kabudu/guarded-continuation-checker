# Watchdog/interlock trace analysis

Imagine nine Boolean latches representing watchdog expiry, interlock activation,
and fault-propagation signals in a controller. Each next-frame latch depends on
four neighbouring current-frame signals through a fixed interlock rule. Engineers
repeatedly ask whether sparse observations from logs, alarms, and injected faults
can coexist in one valid trace—and need the complete trace when the answer is yes.

Run the executable example:

```sh
./target/release/guarded-continuation-checker \
  benchmark-cq-portfolio watchdog4 9 137,1333,7777 50 10 200000 4141414 \
  results/local-watchdog-portfolio.csv
```

Expected behavior:

- `backend=cq-gcc` and `gate_reason=dense-transition`;
- exact agreement with persistent CDCL;
- `witnesses_valid=true`;
- recognition-inclusive speedup above one on the curated release run.

This is representative of repeated bounded model checking and fault diagnosis:
the transition system is compiled once, then many partial trace questions reuse
the same checkpoint-image clauses. It is not a claim about arbitrary CNF.
