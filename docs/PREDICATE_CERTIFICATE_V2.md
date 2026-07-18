# Proof-carrying predicate certificate v2

Certificate v2 is an experimental, deterministic, independently checkable
artifact for bounded terminal bad-output avoidance. It preserves the semantics
of [certificate v1](PREDICATE_CERTIFICATE_V1.md) while replacing exhaustive
projected-input enumeration with concrete witnesses and checked UNSAT
completeness proofs.

V2 is not yet the portfolio default, a production interface, or evidence of a
scholarly novelty claim.

Corrupted-artifact handling, dependency containment and residual availability
limits are specified in the
[certificate v2 reliability boundary](PREDICATE_CERTIFICATE_V2_RELIABILITY.md).

## Query semantics

Given an AIGER model, declared initial latch state, selected bad output, horizon
`h`, and one partial relevant-input constraint vector for every frame `0..h`,
the result is:

- `avoidable`: an allowed concrete input trace reaches a state/input pair at
  frame `h` where the selected bad output is false; or
- `unavoidable`: no allowed concrete input trace can do so.

This is a bounded counterfactual query, not an unbounded safety proof.

## Evidence construction

Each maximal repeated constraint phase records an exact one-step relation and
its deterministic relational power.

For every claimed one-step edge `(source, target)`:

1. a declared-input witness is recorded;
2. direct original-AIG evaluation must reach `target`; and
3. the witness must satisfy the phase constraints.

For each source state, a native Varisat 0.2.2 proof establishes UNSAT for the
one-step AIG CNF under:

- the source-state assignment;
- the phase input constraints; and
- `next != target` for every claimed target.

The proof therefore excludes every omitted target. Witnesses exclude invented
targets. The verifier computes the phase power itself from this checked base
relation.

The terminal safe-state set uses the same dual construction: one direct witness
per claimed safe state, plus one UNSAT proof for a state outside the claimed set
with an allowed input and the bad output false.

The final positive trace is replayed directly through the original AIG. A
negative result is accepted only when the checked composed relation from the
initial state has empty intersection with the checked terminal safe set.

## Canonical text order

The artifact is newline-terminated UTF-8/ASCII text with LF only. Every field
must occur exactly once in the following order; dynamic fields use ascending
numeric indices.

```text
predicate_certificate_version=2
semantics=bounded-terminal-bad-avoidance
proof_format=varisat-native-0.2.2
input_sha256=<64 lowercase hexadecimal digits>
declared_inputs=<decimal>
relevant_inputs=<decimal>
latches=<decimal>
horizon=<decimal>
bad_output=<decimal>
initial_state=<decimal>
result=<avoidable|unavoidable>
phase_count=<decimal>
relevant_0=<declared-input index>
...
phase_0=<start>,<length>,<constraints>
phase_0_base_rows=<hex rows separated by :>
phase_0_powered_rows=<hex rows separated by :>
phase_0_edge_count=<decimal>
phase_0_edge_0=<source>,<target>,<declared input>
...
phase_0_proof_count=<state count>
phase_0_proof_0=<lowercase byte hex>
...
terminal_constraint=<constraints>
terminal_safe_states=<canonical hex bitset>
terminal_witness_count=<decimal>
terminal_witness_0=<state>,<declared input>
...
terminal_proof=<lowercase byte hex>
state_count=<decimal>
states=<comma-separated decimals|->
input_count=<decimal>
inputs=<comma-separated decimals|->
```

Constraint characters are `0`, `1`, or `x`. Relation row `i` is a bitset of
targets reachable from source state `i`. Edges must enumerate every set bit in
source-major, target-minor order. Terminal witnesses must enumerate set bits in
ascending state order. Decimal values have no sign or leading zero; hexadecimal
values are minimal lowercase encodings.

## Fail-closed limits

- relevant inputs: 9–16;
- latches: 1–4, therefore at most 16 states;
- horizon: 0–64;
- certificate file: 16 MiB;
- individual decoded proof: 1 MiB;
- aggregate decoded proofs: 8 MiB;
- phase proofs: exactly one per source state;
- phase edges: at most `states²`;
- terminal witnesses: at most one per state; and
- trace states/inputs: at most `horizon + 1` each.

The parser rejects symlinks, CRLF, truncation, unknown/reordered/duplicate
fields, non-canonical numbers and hex, count mismatches, oversized evidence and
unsupported proof formats before semantic acceptance.

Malformed native proofs can trigger unexpected failures in `varisat-checker`
0.2.2. A structural preflight rejects unsafe dimensions before checking, and a
final unwind boundary converts an unexpected dependency failure to a fail-closed
error.

The producer additionally refuses overwrite and publishes atomically. Proof,
witness, source, phase, ordering, powered-row, terminal, or trace disagreement
is an error; it is never converted into a verification answer.

## Commands

```sh
guarded-continuation-checker certify-aiger-predicate-v2 \
  INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt CERTIFICATE.cert2

guarded-continuation-checker verify-aiger-predicate-certificate-v2 \
  INPUT.aag|INPUT.aig CERTIFICATE.cert2
```

The verifier does not call `PredicateQuotient`, its BDD manager, or producer
caches. It independently recovers AIG support, rebuilds each proof obligation,
checks native proofs through `varisat-checker`, evaluates witnesses directly,
computes relation powers and composition, and validates the final answer.

## Trust and compatibility boundary

The trusted path contains:

- strict v2 parsing and bounds;
- AIGER parsing and support recovery;
- the small one-step Tseitin obligation encoder;
- direct AIG evaluation and relation arithmetic; and
- `varisat-checker` 0.2.2's native-proof checker.

The BDD producer and SAT proof-generating solver are outside the trusted answer
path. The solver and checker currently share the Varisat release family, and the
native proof format is version-bound rather than a cross-tool standard. A
standard proof format plus an independently maintained checker remains required
for checker diversity.

Any incompatible field, order, proof-format, semantic, or limit change requires
a new certificate version. V1 remains supported and remains the portfolio
format until v2 reliability, resource, cost and compatibility gates close.
