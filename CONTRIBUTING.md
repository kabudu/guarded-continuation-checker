# Contributing

Contributions are welcome after the repository is made public.

Before submitting changes:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets
```

The preserved historical experiment harness currently has advisory Clippy
warnings. New continuation-focused code should not add warnings; eliminating
legacy warnings should be submitted separately from scientific changes.

Experimental claims must include raw or summarized results, deterministic seeds,
agreement checks against an independent solver, and witness validation. Negative
results and counterexamples must not be removed from the record.

Do not claim generic SAT improvement or implications for P versus NP without a
formal proof and independent review.
