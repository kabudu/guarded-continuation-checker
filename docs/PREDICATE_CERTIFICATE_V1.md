# Dense predicate certificate v1

This document freezes the first candidate certificate contract for bounded
dense predicate queries. The producer may use BDDs, relation caches and powered
composition. The verifier must not call those producer components: it derives
the claimed relations by exhaustive evaluation of the original AIG within the
certificate's static resource bounds.

The separation is intentional. A production user can distrust the specialised
algorithm while retaining a small deterministic checker whose work is bounded
by at most 16 relevant inputs, 4 latches and horizon 64.

## Semantics

Given an AIGER source, initial latch state, bad-output index, horizon and a
partial relevant-input constraint vector for every frame, the certificate
claims exactly one result:

- `avoidable`: a constrained execution reaches the terminal frame with the bad
  output false; or
- `unavoidable`: every constrained execution reaching the terminal frame has
  the bad output true.

This is a bounded counterfactual query. It is not an unbounded safety proof.

## Canonical text format

The file is UTF-8, newline terminated, contains one `key=value` field per line,
and rejects unknown, missing or duplicate fields. Integers are canonical
unsigned decimal. Booleans are `0` or `1`. Digests are lowercase hexadecimal.
Lists are comma-separated with no whitespace.

Required scalar fields:

```text
predicate_certificate_version=1
semantics=bounded-terminal-bad-avoidance
input_sha256=<64 lowercase hex characters>
declared_inputs=<count>
relevant_inputs=<count>
latches=<count>
horizon=<0..64>
bad_output=<index>
initial_state=<low-latch-bit-first integer>
result=avoidable|unavoidable
phase_count=<count>
```

The relevant-input mapping is:

```text
relevant_0=<declared input index>
...
```

Each maximal run of identical non-terminal constraints is encoded once:

```text
phase_0=<start frame>,<length>,<constraint symbols>,<relation rows>
```

Constraint symbols are `0`, `1`, or `x` in relevant-input order. Relation rows
are lowercase hexadecimal bitsets, one per source latch state, separated by
`:`. Bit `t` in row `s` claims that target latch state `t` is reachable from
source `s` in exactly `length` transitions under that phase constraint.

The terminal constraint and safe-state mask are:

```text
terminal_constraint=<constraint symbols>
terminal_safe_states=<lowercase hexadecimal bitset>
```

For `avoidable`, concrete replay evidence is mandatory:

```text
state_count=<horizon + 1>
states=<comma-separated latch-state integers>
input_count=<horizon + 1>
inputs=<comma-separated complete declared-input u64 integers>
```

For `unavoidable`, `state_count=0`, `states=` is represented by the canonical
sentinel `states=-`, and similarly for inputs. The verifier accepts the claim
only when the composed relation from the declared initial state has empty
intersection with `terminal_safe_states`.

## Independent verification algorithm

1. Parse the source AIGER and bind its SHA-256 digest.
2. Recompute exact combined transition/property input support and compare the
   ordered relevant mapping.
3. Check all static bounds and the declared initial state.
4. For each distinct phase constraint, enumerate every permitted relevant-input
   pattern and every source latch state, evaluate the original AIG, and build a
   one-step relation.
5. Exponentiate that relation by deterministic squaring and compare every
   claimed phase row.
6. Compose phases in frame order.
7. Exhaustively evaluate the terminal bad output to reconstruct the safe-state
   mask and compare it.
8. For `avoidable`, replay every complete input and state through the original
   AIG, including all constraints and the terminal property.
9. For `unavoidable`, prove the composed initial-state targets have no safe
   terminal state.

## Fail-closed limits

- certificate file: 4 MiB;
- declared inputs: 64;
- relevant inputs: 9–16;
- latches: 1–4;
- horizon: 64;
- bad outputs: 128;
- phases: 65;
- relation rows: exactly `2^latches` per phase;
- exhaustive verifier evaluations: at most 80 million;
- no symlinks, non-regular files, unknown fields or trailing data.

Every parse, source-binding, dimensional, arithmetic, resource, semantic or
replay failure is an invalid certificate. It must never be interpreted as an
`avoidable` or `unavoidable` answer.

## Trust and novelty boundary

The checker still trusts the Rust compiler, process environment, AIGER parser,
SHA-256 implementation and its own small exhaustive evaluator. The certificate
does not establish general model-checking certification novelty. Its candidate
contribution is the deterministic binding of bounded predicate projection,
phase powers, static admission and concrete counterfactual replay; that claim
remains subject to the gates in [`NOVELTY_GAP.md`](NOVELTY_GAP.md).
