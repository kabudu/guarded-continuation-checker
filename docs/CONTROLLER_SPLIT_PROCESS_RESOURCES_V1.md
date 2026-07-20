# Controller split process resources v1

Status: single-host baseline plus a hosted Linux measurement gate. Not a
production claim or a cross-host performance comparison.

## Question

What whole-process wall time and peak resident memory does the governed split
workflow consume when one proof-carrying controller is reused across two
independently replaceable plant batches?

The measurement separates four lifecycle operations:

1. certify the shared controller evidence;
2. certify the door-interlock plant result;
3. certify the lock-interlock plant result; and
4. verify both results under one resource policy and one controller admission.

This separation prevents controller construction cost from being silently
charged to every replacement and prevents verification-only measurements from
hiding producer cost.

## Reproduce

Build the current release executable and run three trials:

```sh
cargo build --release --locked
TRIALS=3 scripts/benchmark-controller-split-process-resources.sh \
  target/release/guarded-continuation-checker \
  /tmp/controller-split-process-resources.csv
```

The script supports macOS and Linux. It uses the platform's `/usr/bin/time`,
records the operating system and architecture as separate fields, converts GNU
time's KiB maximum-resident-set measurement to bytes, and refuses to overwrite
an existing result. It accepts at most 20 trials.

Every command output is checked before its measurement is retained. The plant
rows must each certify exactly one member. The governed verification must emit
exactly two batch rows, one controller admission, and the two expected UNSAFE
answers. The CSV also records the deterministic evidence bytes associated with
each operation.

## First observation

[`results/controller-split-process-resources-arm64-v1.csv`](../results/controller-split-process-resources-arm64-v1.csv)
contains three release-build trials from one Darwin arm64 host. The maximum
observed peak RSS was 20,824,064 bytes for controller certification and
8,814,592 bytes for governed verification. The observed verification wall time
was 0.03 seconds in all three trials. The 248,889-byte controller evidence and
the 365-byte and 476-byte plant results match the retained structural
acceptance evidence.

These values describe one host at one point in time. Scheduler state, toolchain,
kernel, hardware, and measurement resolution affect them. They must not be
used as portable thresholds, routing inputs, or evidence that one tool is
faster than another.

## CI contract

Ordinary Linux CI builds the release executable, runs the same four-operation
measurement, and requires three complete trials with non-negative elapsed time,
positive peak RSS, exact answers, and explicit Linux plus architecture labels.
CI prints the observed rows but does not compare them byte-for-byte with the
arm64 CSV. Deterministic structural acceptance remains a separate exact-diff
gate.

This closes the missing whole-process peak-RSS measurement mechanism for the
two-batch split fixture. It does not close compatibility through a later tag,
independent acceptance, broader workload coverage, or production readiness.
