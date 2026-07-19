# Proof-carrying event-contract certificate v3

Certificate v3 is an experimental deterministic artifact for exact bounded
terminal bad-output avoidance under named CNF event contracts. It binds an AIGER
model and the original event-contract file to independently checked relation,
composition, terminal-set, answer, and optional concrete-trace evidence.

V3 is not yet part of the stable predicate CLI, Rust API, or production
portfolio. Its byte contract is frozen for the experiment branch, but release
compatibility begins only if the remaining reliability and integration gates
pass.

## Query semantics

Given an AIGER model, declared initial latch state, selected bad output, bounded
event contract, and horizon `h`, the result is:

- `avoidable`: an event-contract-admissible concrete input trace reaches a
  state/input pair at frame `h` where the selected bad output is false; or
- `unavoidable`: no admissible concrete input trace can do so.

This is a bounded counterfactual statement, not an unbounded safety proof. An
inconsistent event contract has no avoiding execution and therefore returns
`unavoidable`; the completeness evidence proves the empty relation rather than
silently assuming the contract is satisfiable.

## Source binding

The artifact records two SHA-256 digests:

1. the exact AIGER input bytes; and
2. the exact canonical named event-contract bytes.

The verifier reparses both supplied files, independently recovers relevant AIG
support, and requires every phase boundary and parsed predicate to equal the
certificate. A digest match alone is not treated as semantic validation.

## Evidence

Each phase contains its named-CNF predicate in canonical projected-input form,
the exact one-step relation, its declared relational power, concrete input
witnesses for every claimed edge, and one native UNSAT completeness proof per
source state.

The verifier:

1. evaluates every edge witness directly against the source AIG and CNF;
2. rebuilds and checks every omitted-target UNSAT obligation;
3. recomputes each relation power and composes phases itself;
4. checks every terminal safe-state witness and the omitted-safe-state proof;
5. replays an `avoidable` trace directly through the AIG and contract; and
6. accepts `unavoidable` only when the independently composed reachable set has
   empty intersection with the checked terminal safe-state set.

The producer BDD, its caches, and its powered rows are outside the trusted
answer path.

## Canonical field order

The certificate is newline-terminated UTF-8/ASCII text with LF only. Every
field occurs once in the following order; indexed fields use ascending decimal
indices.

```text
event_contract_certificate_version=3
semantics=bounded-named-cnf-terminal-bad-avoidance
proof_format=varisat-native-0.2.2
input_sha256=<64 lowercase hexadecimal digits>
contract_sha256=<64 lowercase hexadecimal digits>
declared_inputs=<decimal>
relevant_inputs=<decimal>
latches=<decimal>
horizon=<decimal>
bad_output=<decimal>
initial_state=<decimal>
result=<avoidable|unavoidable>
phase_count=<decimal>
relevant_0=<declared input index>
...
phase_0=<start>,<length>
phase_0_clause_count=<decimal>
phase_0_clause_0=<projected literals separated by |>
...
phase_0_base_rows=<hex rows separated by :>
phase_0_powered_rows=<hex rows separated by :>
phase_0_edge_count=<decimal>
phase_0_edge_0=<source>,<target>,<declared input>
...
phase_0_proof_count=<state count>
phase_0_proof_0=<lowercase byte hex>
...
terminal_clause_count=<decimal>
terminal_clause_0=<projected literals separated by |>
...
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

Positive literals are decimal projected-input indices; `!` marks negation.
Literals are sorted by `(input, sign)` and clauses are strictly sorted. Decimal
values have no sign or leading zero. Hexadecimal values are minimal lowercase
encodings. Edges are source-major and target-minor; terminal witnesses use
ascending state order.

## Hard limits

- declared inputs: 1 to 64;
- relevant inputs: 9 to 16;
- latches: 1 to 4, therefore at most 16 states;
- horizon and phases: 1 to 64;
- clauses per predicate: 64;
- literals per clause: 16;
- certificate file: 32 MiB;
- individual decoded proof: 1 MiB;
- aggregate decoded proofs: 8 MiB;
- phase edges: at most `states squared`;
- phase proofs: exactly one per source state;
- terminal witnesses: at most one per state; and
- trace states and inputs: at most `horizon + 1` each.

The parser reads through a bounded opened file handle. Unix builds use no-follow
open semantics in addition to initial symlink rejection. It rejects invalid
UTF-8, CRLF, missing final newline, reordering, unknown fields, noncanonical
numbers or clauses, count mismatches, oversized evidence, and unsupported
formats before semantic verification.

Native proof streams receive structural preflight before `varisat-checker` and
unexpected checker failures are converted to fail-closed verification errors.
The producer refuses overwrite and publishes atomically.

## Commands

```sh
guarded-continuation-checker certify-aiger-event-contract-v3 \
  INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt CERTIFICATE.cert3

guarded-continuation-checker verify-aiger-event-contract-certificate-v3 \
  INPUT.aag|INPUT.aig CONTRACT.txt CERTIFICATE.cert3
```

The verifier does not invoke the event-contract query producer. Logical answers
and operational errors remain distinct. No verifier error is converted into an
`avoidable` or `unavoidable` answer.

## Cost result

Release-mode measurements were taken on 19 July 2026 using Rust 1.97.0 on Apple
Silicon. Values are medians of ten raw trials. The exact CDCL column is a solving
control, not equivalent work: v3 verification checks a reusable proof artifact,
all relation edges, phase powers, terminal evidence, and any concrete trace.

| Contract | Result | Generation | Verification | Exact CDCL | V3/CDCL | Artifact | Proofs |
|---|---|---:|---:|---:|---:|---:|---:|
| Interrupt priority | avoidable | 11.301 ms | 0.580 ms | 0.256 ms | 2.26x | 17,674 B | 9 |
| Actuator interlock | avoidable | 17.125 ms | 0.676 ms | 0.279 ms | 2.43x | 27,838 B | 17 |
| Robot recovery | avoidable | 56.709 ms | 1.419 ms | 0.571 ms | 2.48x | 84,033 B | 33 |
| Actuator fixed-input | unavoidable | 9.789 ms | 0.288 ms | 0.040 ms | 7.23x | 11,329 B | 9 |

All 40 rows agreed with exact CDCL and passed independent verification. The
negative performance result is retained: v3 verification is slower than solving
these individual queries. Its value is deterministic, independently replayable
assurance and potential reuse across trust boundaries, not universal query
speed.

Raw evidence and reproduction instructions are under
[`results/event-contract-certificate-v3-cost`](../results/event-contract-certificate-v3-cost/README.md).

## Trust and prior-art boundary

The trusted path contains strict parsing and bounds, AIGER parsing and support
recovery, the one-step Tseitin obligation encoder, direct AIG evaluation,
relation arithmetic, and `varisat-checker` 0.2.2. The producer and proof-generating
solver are untrusted for answer acceptance.

SAT proof carrying, BDD symbolic composition, CNF assumptions, contracts, and
witness replay are established techniques. V3 is evidence for the repository's
narrow combined candidate contribution, not proof of scholarly novelty. A
maintained external proof-checker comparison, prior-art search for the complete
combination, and independent review remain mandatory.

## Remaining admission gates

Before v3 can enter a stable portfolio:

1. complete deterministic mutation, truncation, source-substitution, proof-swap,
   resource, and process-isolation testing; and
2. pass the frozen CLI/Rust API, portfolio, and hard-resource regressions on the
   supported Linux release path.

The maintained external proof-checker gate is now closed by the
[CaDiCaL and DRAT-trim baseline](EXTERNAL_EVENT_CONTRACT_PROOF_BASELINE.md).
The answer-balanced cost, timing-free static admission, exact fallback, and
release-candidate API gates are closed by
[`EVENT_CONTRACT_CLI_V1.md`](EVENT_CONTRACT_CLI_V1.md) and the retained
[`event-contract-certificate-v3-balanced-v1`](../results/event-contract-certificate-v3-balanced-v1/README.md)
cohort. Compatibility history does not begin until the first tagged release.
The release-candidate surface also passes the full Rust 1.97 Linux test job,
public RTL corpus, and dependency audit in
[hosted run 29667786512](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29667786512).
