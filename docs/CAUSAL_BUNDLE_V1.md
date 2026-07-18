# Causal evidence bundle v1

This document defines the on-disk contract produced by
`explain-aiger-counterexample` and consumed by
`verify-aiger-causal-bundle`. Verification is exact for the bounded Boolean
model and intervention semantics described in [CAUSAL_ANALYSIS.md](CAUSAL_ANALYSIS.md).

## Directory contract

The output is a newly created directory containing exactly three regular files:

```text
causal-certificate.txt
causal-manifest.txt
causal-metrics.csv
```

The producer refuses to overwrite an existing output path. It writes and
verifies a sibling staging directory before one atomic rename publishes the
bundle. A failed generation does not publish a successful-looking bundle.
Atomic no-clobber publication is supported on Linux, Android, and macOS and
fails closed on other operating systems.

All files are UTF-8 text, no file may exceed 1 MiB, and symbolic links,
subdirectories, missing files, and additional files are rejected.

## Manifest

`causal-manifest.txt` is an unordered, duplicate-free `key=value` document with
exactly these fields:

```text
causal_bundle_version=1
input_sha256=<64 lowercase or uppercase hexadecimal characters>
certificate=causal-certificate.txt
certificate_sha256=<SHA-256 of causal-certificate.txt>
metrics=causal-metrics.csv
metrics_sha256=<SHA-256 of causal-metrics.csv>
```

The verifier recomputes all three digests. The bundle is therefore bound to one
exact AIGER source and detects accidental or unreflected evidence modification.
The manifest is not a digital signature and does not establish who created it.

## Certificate

`causal-certificate.txt` uses duplicate-free `key=value` lines. Version 1 has
eleven fixed fields followed by exactly `cause_count` indexed fields:

```text
causal_certificate_version=1
input=<descriptive source path>
input_sha256=<source SHA-256>
requested_horizon=<positive decimal integer>
bad_frame=<decimal integer not exceeding requested_horizon>
bad_output=<zero-based output index>
bad_output_name=<name resolved from the source model>
semantics=minimal-sufficient-input-segments
minimality=1-minimal
candidate_count=<decimal integer from 1 through 512>
cause_count=<decimal integer not exceeding candidate_count>
cause_0=<input>,<start_frame>,<end_frame>,<0-or-1>
...
```

The verifier does not trust the declared result. It reparses the digest-bound
model, rederives its earliest counterexample, reconstructs the full
constant-segment vocabulary, checks every retained segment belongs to that
vocabulary, proves the retained set forces the same failure, and proves that
removing each retained segment permits the target output to be false at the
certified frame.

## Metrics

`causal-metrics.csv` contains exactly the following header and one data row:

```text
input_sha256,requested_horizon,bad_frame,bad_output,candidates,causes,queries,cq_admitted,cq_bound_bits,cq_peak_classes,cq_compile_ns,cq_query_ns,persistent_cdcl_ns,fresh_cdcl_ns,cq_query_speedup,cq_amortized_speedup,agreement,certificate_valid,status
```

Identity and count columns must agree with the certificate. Integer timing
columns must parse as non-negative integers; ratios must be finite and
non-negative; `queries` must be positive; and the terminal fields must be
`true,true,ok`. Timings are observations, not reproducible proof claims. The
certificate verifier establishes correctness independently of them.

## Resource limits

- Input: existing ASCII AIGER limit of 256 MiB.
- BMC encoding: at most two million variables.
- Candidate input segments: at most 512.
- Conservative causal query work: at most 250 million clause/variable units.
- CQ eligibility: at most 256 variables and 4,096 clauses.
- User-supplied CQ frontier admission bound: at most 20 bits.

CQ rejection is not an analysis failure. Fresh and persistent CDCL remain the
exact reference paths; any disagreement between admitted CQ and either CDCL
path aborts publication.
