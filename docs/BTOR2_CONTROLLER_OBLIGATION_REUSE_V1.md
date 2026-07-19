# BTOR2 controller-obligation reuse v1

## Status

This document predeclares the experiment for reusing one independently checked
controller obligation across multiple plant variants. The obligation format and
standalone producer and verifier are implemented. The exact, bounded,
mixed-answer naive-bundle baseline is executable through the public Rust API.
Compact batch composition, independent verification, a canonical artifact, and
a manifest-driven CLI are implemented. The first local strong-baseline result
passes for fully admitted batches and fails for a 25 percent fallback control.
Product validity, maintained external controls, and cross-platform replication
remain open. This is not yet a novelty or production-readiness claim.

## Reusable obligation

The v1 obligation binds the exact SHA-256 digest of one controller source and
records its controller-facing interface:

- reset and sensed-velocity inputs;
- braking state and combinational brake output;
- sensed-velocity width; and
- nonzero braking threshold.

The producer recognises a resettable, initially false braking latch whose output
is the old latch value OR an unsigned velocity-threshold comparison. The
verifier independently reparses the controller and reconstructs the transition,
initialisation, width, output, and threshold claims. A changed source or claim is
rejected.

The canonical text format is versioned, bounded to 2 KiB, source-bound, fixed
order, LF-only, and round-trip checked. Every truncation and every single-byte
mutation is tested. The Rust API is shell-free and the CLI supports standalone
production and verification.

## Strong comparison baseline

The experiment will not compare only with repeated process startup. Its baseline
must parse and validate the controller once in-process, then use straightforward
ordinary component-certificate bundling for every plant query. This removes
avoidable parsing and launch overhead from the claimed benefit.

`produce_naive_component_batch` and `verify_naive_component_batch` freeze that
baseline. They preserve ordinary specialised and exact-fallback certificates,
bind controller digest, member count, order, sources, contracts, and horizons,
parse the immutable controller model exactly once, and share that parsed model
without a per-member model clone. They admit at most 64 members. The retained
test combines specialised SAFE,
fallback SAFE, and fallback UNSAFE members and rejects query reordering.

The reusable batch must preserve:

- the original controller, plant, contract, and horizon bindings;
- exact SAFE and UNSAFE answers;
- direct exact composed-search fallback for rejected or intersecting members;
- deterministic member ordering and canonical encoding;
- independent verification of the shared obligation and every member; and
- fail-closed resource limits for member count, total bytes, and total work.

## Predeclared gates

| Gate | Required result |
|---|---|
| Exactness | Every answer agrees with ordinary component verification and maintained controls |
| Mixed answers | One batch contains specialised SAFE, exact-fallback SAFE, and exact-fallback UNSAFE members |
| True reuse | Controller source and controller obligation are checked once, not once per member |
| Strong artifact baseline | Total reusable artifact bytes are lower than one shared controller obligation plus naively bundled ordinary certificates |
| Strong checking baseline | Median in-process batch checking is materially faster than a parse-once straightforward baseline |
| Break-even | A deterministic member count identifies when production plus checking amortises |
| Hostile input | Source, obligation, member, order, count, truncation, and size tampering fail closed |
| External baseline | BTOR2Tools parses every source and pinned Bitwuzla covers obligation-equivalent arithmetic |
| Product validity | At least one public, unmodified firmware or robotics design family exercises the interface |

Failure of either strong baseline falsifies the performance-reuse hypothesis.
Passing only fixture, artifact-size, or process-startup comparisons is
insufficient for a novelty claim.

## First strong-baseline result

The release-mode benchmark alternates the base and fast braking plants, runs 101
interleaved trials per row, and compares in-process verification. The baseline
parses the controller once and shares its immutable model. The challenger
independently verifies one controller obligation, then parses and checks only
the plant side of each admitted member.

For fully admitted batches, the reusable artifact breaks even at 2 members and
is 34.0 percent smaller at 64 members. Median verification is faster in every
row. Across five repeated runs, the 64-member verification ratio stayed between
0.866 and 0.885, which is an 11.5 to 13.4 percent reduction. Production is
slower because the current producer first builds ordinary certificates and then
projects them into compact members. One to five subsequent verifications repay
that cost depending on batch size. Batches of 4 or more repaid it within two
verifications in every retained run.

The hostile control replaces 25 percent of members with exact UNSAFE fallback
certificates. It fails the artifact gate by 20.5 to 28.4 percent and shows no
stable verification improvement. Therefore, v1 must not be selected universally.
The implemented portfolio selects reusable encoding only when at least two
members are present and every member is admitted. Singleton and mixed batches
are converted back to ordinary component certificates without re-solving.
Verification rejects a reusable route that violates this static gate. The rule
uses certificate structure only and performs no timing calibration.

The public APIs are `produce_component_batch_portfolio`,
`verify_component_batch_portfolio`, `encode_component_batch_portfolio`, and
`decode_component_batch_portfolio`. The portfolio artifact is versioned,
canonical, bounded to 65 MiB, and retains the nested certificate limits.

The self-service CLI accepts a canonical manifest containing normalized
relative paths:

```text
component_batch_manifest_version=1
member_count=2
plant_path=motion-plant-v1.btor2
contract_path=braking-motion-contract-v1.txt
horizon=254
plant_path=motion-plant-v1.btor2
contract_path=braking-motion-contract-v1.txt
horizon=255
status=complete
```

Create and independently verify an artifact with:

```sh
guarded-continuation-checker check-btor2-component-batch \
  braking-controller-v1.btor2 batch.txt batch.component-batch
guarded-continuation-checker verify-btor2-component-batch \
  braking-controller-v1.btor2 batch.txt batch.component-batch
```

The retained simulated self-service acceptance covers the admitted reusable
route, the mixed ordinary route, query drift, controller drift, and artifact
mutation. All five cases behave as predeclared in
[`results/btor2-component-batch-acceptance-v1.csv`](../results/btor2-component-batch-acceptance-v1.csv).
This is repository-controlled evidence, not independent partner acceptance.

## Compatibility and migration

The contract, ordinary component certificate, controller obligation, reusable
batch, portfolio, and manifest carry independent format versions. Version 1
field order, number spelling, LF termination, route semantics, and digest
bindings are frozen. A producer change that alters canonical bytes or semantics
must introduce a new format version rather than reinterpret version 1.

The portfolio decoder is self-identifying. A reusable route remains the exact
`reusable_component_batch_version=1` artifact, so portfolio wrapping does not
erase the measured size benefit. An ordinary route starts with
`component_batch_portfolio_version=1`. Version 1 fingerprints for the admitted
and mixed public fixtures are retained as unit-test gates. When a successor is
introduced, its decoder must coexist with version 1 for at least two subsequent
minor releases, and the migration notes must state whether re-encoding preserves
the verified result, source bindings, and member order.

The retained data is in
[`results/btor2-component-reuse-v1.csv`](../results/btor2-component-reuse-v1.csv).
Reproduce it with:

```sh
scripts/run-btor2-component-reuse-benchmark.sh all /tmp/component-reuse.csv
```

## Closest prior art and claim boundary

Reusable component assumptions and guarantees are established compositional
verification practice. Relevant prior work includes:

- [Compositional Synthesis of Modular Systems](https://link.springer.com/article/10.1007/s11334-022-00450-w), which constructs per-process certificates that capture interfaces;
- [Learning Assumptions for Compositional Verification of Timed Automata](https://link.springer.com/chapter/10.1007/978-3-031-37706-8_3), which learns reusable environment assumptions;
- [Towards Compositional Verification for Modular Robotic Systems](https://arxiv.org/abs/2012.01648), which applies component contracts to modular robotic software;
- [A Framework for Proof Certificates in Finite State Exploration](https://arxiv.org/abs/1507.08716), which independently checks reachability and non-reachability evidence; and
- [Introducing Certificates to the Hardware Model Checking Competition](https://link.springer.com/chapter/10.1007/978-3-031-98668-0_14), which independently checks hardware witness circuits.

Therefore, controller reuse, assume-guarantee composition, source digests,
canonical certificates, and independent checking are not individually novel.
The only candidate claim is a measured, exact proof decomposition that reuses a
controller-local obligation across real embedded variants and beats strong
straightforward composition baselines. That claim remains unproven.
