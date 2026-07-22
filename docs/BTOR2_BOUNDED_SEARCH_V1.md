# BTOR2 bounded search certificate v1

Status: retained experimental format. Input-dependent bad properties use the
additive [v2 format](BTOR2_BOUNDED_SEARCH_V2.md); v1 encoding and verification
remain unchanged.

This experimental layer answers an actual bounded reachability question:

> Does any sequence of the admitted Boolean input make the selected bad
> property true at a frame from zero through the requested horizon?

It handles both answers. `UNSAFE` carries an input witness that the verifier
replays from the source initial state. `SAFE` carries every canonical reachable
state layer. The verifier reconstructs both input successors of every state,
checks exact equality with the next layer, and checks that no layer contains the
bad property.

This is exact explicit-state bounded model checking. It is not claimed as a new
algorithm, a production interface, or a solution for state explosion.

## Admitted boundary

Version 1 requires:

- the strict BTOR2 word-core syntax;
- exactly one 1-bit input;
- a bad property with no direct or indirect input dependency;
- no constraints;
- horizon at most 256;
- at most 65,536 states per layer and 262,144 states in the certificate;
- at most 20,000,000 node-root-steps, including the state-expression count; and
- a canonical certificate no larger than 16 MiB.

Every unsupported feature or exceeded bound is an error. It is not converted
into `SAFE`, `UNSAFE`, or an approximate state layer.

## Self-service commands

```sh
cargo run --release -- \
  search-btor2 examples/btor2/watchdog-counter-v1.btor2 \
  13 2 /tmp/watchdog-safe.search-cert

cargo run --release -- \
  verify-btor2-search examples/btor2/watchdog-counter-v1.btor2 \
  /tmp/watchdog-safe.search-cert
```

Changing the horizon from 2 to 3 produces an `UNSAFE` certificate whose input
witness contains three false reset controls.

## Evidence and limitations

The watchdog cohort covers both answers. The non-affine saturating timer is
`SAFE` through bound 254 and `UNSAFE` at bound 255, showing the search is not
dependent on affine recognition. Official BTOR2Tools checks concrete unsafe
witness semantics for the source models. Pinned Bitwuzla 0.9.1 independently
checks the watchdog reachability unrolling as UNSAT at bound 2 and SAT at bound
3.

The reproducible [six-query cohort](../results/btor2-search-cohort-v1.md)
records both boundary answers for the watchdog, actuator, and saturating timer.
SAFE certificates grow to 505,396 and 802,525 bytes on the longer models, while
their UNSAFE witnesses remain below 500 bytes. This is the measured compression
target for the next word-composition experiment.

Explicit reachable-layer certificates, graph reachability, bounded model
checking, witness replay, BTOR2, and SMT bit-vectors are established. The value
of this layer is a source-bound, fail-closed integration and a reference
fallback for future composition research. It does not establish novelty. The
next research question is whether word-level composition can replace large
explicit layers while retaining a checker that proves the same complete
successor obligation.
