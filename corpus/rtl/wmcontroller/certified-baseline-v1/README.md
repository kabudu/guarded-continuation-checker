# Certified baseline AIGER exports v1

These six AIGER 1.9-compatible ASCII models encode the public washing
controller, repository-authored physical plant, one selected plant property,
and the original horizon of 32. A six-bit frame counter checks frames 0 through
32 and then enters an absorbing completed state with a false bad output.
Unbounded safety of each export is therefore equivalent to its bounded source
query.

Regenerate into a new directory with:

```sh
cargo run --locked --release \
  --example export_washing_controller_certified_baseline -- OUTPUT_DIR
```

The generator refuses to overwrite an existing directory. `manifest-v1.txt`
binds source and generated inputs, property indices, expected answers, shortest
bad frames, and the six exported model digests. The integration test
`controller_plant_bounded_aiger_api` independently parses and explores the
exports. It does not call the exporter's evaluator when checking answers or
shortest bad frames.
