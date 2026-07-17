# Hostile-RTL isolation profile v1

Isolation profile v1 places CQ-SAT/GCC and Yosys inside a fail-closed Docker
boundary before parsing partner RTL. It is the required baseline for inputs that
are not already trusted by the evaluation worker owner.

## Command contract

```sh
scripts/isolated-rtl-evaluation.sh \
  CQ_LINUX_BINARY PROJECT_CONFIG OUTPUT_DIR
```

The binary must be an executable Linux/amd64 build. The project config and all
of its relative sources/includes are mounted read-only. `OUTPUT_DIR` and its
sibling `.isolation-report.txt` must not exist and may not be inside the input
tree. The pinned image must be installed before invocation; runtime pulling is
disabled.

Exit meanings preserve firmware CLI contract v2:

- `0`: validated SAFE evidence.
- `1`: validated UNSAFE evidence with a counterexample.
- `2`: input, isolation, tool, or validation failure. This is never SAFE.

## Enforced boundary

Every run uses the immutable image
`hdlc/yosys@sha256:58c0c80e41fd96b4b90da53c730aa3c43051f0cf2a6c6e336bd012281479df22`
and requires a Linux Docker daemon with seccomp enabled. The container has:

- no Docker network;
- a read-only root filesystem and input mount;
- only the dedicated output directory writable;
- the caller's numeric, non-root UID/GID;
- all Linux capabilities dropped;
- `no_new_privileges` and seccomp filtering;
- at most 128 processes;
- 3 GiB memory with swap disabled;
- two CPUs;
- a 300-second outer evaluation deadline and 30-second probe/validation deadlines;
- 1,024 file descriptors;
- a 64 MiB `noexec,nosuid,nodev` temporary filesystem;
- no Docker socket, host devices, credentials, or unrelated host paths.

Before parsing RTL, an adversarial runtime probe verifies the firmware CLI and
artifact schema contract, effective UID, capability mask, `no_new_privileges`,
seccomp mode, cgroup-v2 memory/swap/process/CPU
limits, absence of an IPv4 route or non-loopback IPv6 route, root/input write
failure, and output write success. Any mismatch exits 2 before Yosys starts.

The host wrapper tracks each container by an isolated CID file. At its deadline
it kills that specific container and retains any partial output for incident
analysis; an inner process cannot defeat the outer watchdog. An interrupt or
termination signal kills the active container, removes private control files,
and exits with tool-failure status 2.
CI exercises this exact path with a 60-second container command and a one-second
deadline. The internal watchdog must return 124; the public self-test deliberately
returns tool-failure exit 2 and no evaluation output, so it cannot be mistaken
for a SAFE result.

After CQ returns 0 or 1, a second read-only container validates the schema-v4
bundle. The wrapper compares validated status with the original exit and writes
an isolation report beside—not inside—the immutable evidence bundle.

## Trust boundary and limitations

The Docker daemon, host kernel, pinned image, mounted CQ binary, wrapper script,
and artifact destination remain trusted. A container reduces exposure to a Yosys
parser compromise but is not a virtual-machine boundary. Use a disposable VM as
well when RTL is actively hostile, the partner requires kernel isolation, or the
worker is multi-tenant.

The pinned image contains Yosys 0.36+42. It is retained because its exact digest
has passed the public corpus and isolation suite; it is not represented as the
latest Yosys. Projects requiring newer syntax need a separately reviewed,
digest-pinned image that passes the same corpus and runtime probes before this
script's image constant may change.

The sibling isolation report is operational evidence, not part of artifact
schema v4 and not cryptographically authenticated by it. Retain its digest in a
separately trusted deployment record alongside the bundle manifest digest.
