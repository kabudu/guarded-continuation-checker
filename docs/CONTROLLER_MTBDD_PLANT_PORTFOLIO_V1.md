# Controller MTBDD plant portfolio v1

Status: experimental, deterministic, bounded, and available through the public
Rust library and file CLI. It is not a stable release interface.

## Purpose

The portfolio preserves one ordered controller and plant-property batch across
the MTBDD backend's static resource boundary. It uses no timing, calibration,
learned threshold, or trial execution:

1. Produce the exact controller MTBDD when its frozen state, input, output,
   assignment, terminal, and node limits admit the query.
2. Fall back to direct exact controller evaluation only when production returns
   the explicit boundary-limit, terminal-limit, or node-limit error.
3. Propagate every malformed-model, invalid-query, or semantic error without
   fallback.

Both routes compute the same bounded product semantics and return complete SAFE
or UNSAFE member results. The direct route enumerates omitted controller inputs
and accepts them only when they do not affect the selected exact outcome.

## Artifact and verification

The outer `GCCMPP01` artifact binds:

- format version 1;
- selected backend and exact admission reason;
- ordered relevant controller inputs and observed outputs; and
- either one `GCCMPA01` MTBDD batch or one `GCCDPA01` direct batch.

The outer and embedded artifacts each carry SHA-256 integrity trailers and stay
within the existing 16 MiB controller-plant artifact limit. Every member binds
the controller and plant source digests, wiring, initial states, bad output,
horizon, answer, and unsafe trace when present.

Verification checks the supplied boundary and every ordered member before
independently replaying the selected backend. On the direct route it reruns
MTBDD admission and requires the same static rejection reason. A direct payload
for an admitted MTBDD query is therefore rejected as a downgrade.

## CLI

The CLI reuses the canonical controller MTBDD plant manifest and its bounded,
normalized, no-follow file handling:

```text
guarded-continuation-checker controller-plant-portfolio-cli-version
guarded-continuation-checker certify-controller-plant-portfolio MANIFEST.txt OUTPUT.controller-plant
guarded-continuation-checker verify-controller-plant-portfolio MANIFEST.txt INPUT.controller-plant
```

Production creates a new file and never overwrites an existing path. Both
commands print the backend, reason, aggregate counts, resource metrics, and one
stable result line per member. Verification requires the manifest boundary and
every member query to match the artifact exactly.

Rust integrations can use `ControllerPlantPortfolioTool`. It discovers and
validates the complete portfolio capability contract, invokes the producer or
verifier without a shell, applies the shared bounded execution policy, and
returns typed backend, route, aggregate, member, and invocation results.

## Retained tests

The admitted fixture selects MTBDD and returns one SAFE and one UNSAFE member.
A seven-latch controller exceeds the six-state-bit MTBDD limit, selects direct
exact evaluation, and returns the same answer pair. Tests also reject a forced
downgrade, boundary drift, malformed controller input, every truncation, and
every single-byte mutation of the retained fallback artifact.

The self-service acceptance harness exercises both routes in fresh processes:

```sh
cargo build --release --locked
scripts/run-controller-plant-portfolio-acceptance.sh \
  target/release/guarded-continuation-checker \
  target/controller-plant-portfolio.csv
```

Its stable six-case result is retained at
`results/controller-plant-portfolio-acceptance-v1.csv` and reproduced on the
Linux CI worker. This is simulated external-style acceptance, not independent
partner evidence.

## Claim boundary

This closes a library-level integrity gap: static specialisation no longer
drops a valid bounded query merely because the MTBDD representation exceeds a
frozen resource limit. Exact portfolio routing and fallback are established
engineering techniques. This result is neither a novelty claim nor evidence of
better runtime. Cross-platform replication, compatibility history, independent
acceptance, and external plant provenance remain open.
