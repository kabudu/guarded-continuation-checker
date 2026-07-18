# Counterfactual portfolio v1

The counterfactual portfolio is a bounded, exact, self-service interface for
asking whether an AIGER bad output can be avoided while preserving a partial
input transcript. It uses the proof-carrying dense predicate backend only when
the versioned static admission rule accepts the query. Every static rejection
and recoverable predicate resource failure uses persistent CDCL on the identical
model, horizon, output and transcript.

This is an evaluation interface, not a production-readiness claim and not an
unbounded safety proof.

## Command

```sh
continuation-quotient-sat verify-aiger-counterfactual \
  INPUT.aag|INPUT.aig OUTPUT_INDEX TRANSCRIPT.txt EXPECTED_QUERIES \
  REPORT.txt CERTIFICATE.cert
```

The command refuses to overwrite either output path. `EXPECTED_QUERIES` is an
unsigned workload declaration used only by static backend admission; it cannot
change the logical answer. The transcript follows
[`Dense predicate certificate v1`](PREDICATE_CERTIFICATE_V1.md): one line per
frame, relevant inputs in canonical support order, and characters `0`, `1`, or
`x` only.

Exit status zero means an exact answer was produced and the report was
published atomically. A specialised-backend verification disagreement, malformed
input, unsupported dimensions, output collision, or publication failure returns
nonzero. Certificate verification failures are never hidden by CDCL fallback.

## Backends

| `backend` | `reason` | Evidence |
|---|---|---|
| `predicate-certificate` | `static-admission` | Certificate v1 exists and was independently verified before report publication |
| `persistent-cdcl` | `static-rejection` | No certificate is emitted; exact CDCL solves the original query |
| `persistent-cdcl` | `predicate-resource-fallback` | Predicate construction exceeded its bounded resources before producing an answer; exact CDCL solves the original query |

The v1 admission rule uses only relevant-input width, latch count, horizon and
declared expected reuse. It performs no per-formula timing calibration.

## Report schema

The LF-terminated report contains each key exactly once in this order:

```text
counterfactual_portfolio_version=1
input_sha256=<64 lowercase hexadecimal digits>
bad_output=<decimal>
horizon=<decimal>
relevant_inputs=<decimal>
latches=<decimal>
expected_queries=<decimal>
admitted=<0|1>
backend=<predicate-certificate|persistent-cdcl>
reason=<static-admission|static-rejection|predicate-resource-fallback>
result=<avoidable|unavoidable>
certificate=<present|->
certificate_verified=<0|1>
gate_ns=<decimal>
backend_ns=<decimal>
verifier_ns=<decimal>
status=ok
```

Timing fields are diagnostic monotonic-clock measurements, not logical
evidence. `result=avoidable` means at least one concrete input completion keeps
the selected bad output false at the terminal frame. `result=unavoidable`
means no such completion exists under the transcript.

The certificate path is intentionally not embedded in the report. Consumers
bind the sibling artifact through the source digest and certificate contents,
avoiding path interpretation in evidence processing.

## Fail-closed publication

An admitted certificate is independently checked before the report is written.
If checking or preflight agreement fails, the certificate is removed and no
report is published. If report publication fails after a certificate was
verified, the certificate is removed. CDCL paths never create a certificate.

## Independent report verification

```sh
continuation-quotient-sat verify-aiger-counterfactual-report \
  INPUT.aag|INPUT.aig TRANSCRIPT.txt REPORT.txt CERTIFICATE.cert
```

The verifier is a strict consumer of the frozen field order, canonical decimal
and Boolean encodings, LF termination, 16 KiB size limit and regular-file rule.
It rejects unknown, missing, reordered, duplicate, non-canonical and malformed
fields. It recomputes the source digest, support, dimensions and static
admission decision.

For an admitted predicate result it independently verifies certificate v1 and
requires its output, result and fully expanded transcript to match the report.
For either CDCL reason it requires the certificate path not to exist and
re-solves the identical query with persistent CDCL. A resource-fallback report
is valid only for a statically admitted query; a static-rejection report is
valid only for a rejected query. Timing fields are parsed but never trusted as
evidence.

## Compatibility

The command and report are frozen as version 1 for research evaluation.
Incompatible field, ordering, semantic, or admission changes require a new
version. The implementation does not yet provide a stable library API,
deprecation window or signed distribution; those remain production-readiness
gates.
