# BTOR2 controller-obligation reuse v1

## Status

This document predeclares the experiment for reusing one independently checked
controller obligation across multiple plant variants. The obligation format and
standalone producer and verifier are implemented. The exact, bounded,
mixed-answer naive-bundle baseline is also executable through the public Rust
API. Compact batch composition and its comparative measurements remain in
progress. This is not a novelty or production-readiness claim.

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
and admit at most 64 members. The retained test combines specialised SAFE,
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
