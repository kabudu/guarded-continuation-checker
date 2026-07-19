# Controller MTBDD process resource baseline v1

Status: retained negative and positive process-level evidence on one arm64
development host. A hosted Linux replication remains required.

## Question

How does a fresh GCC consumer of the six-property controller MTBDD artifact
compare with the maintained single-session SymbiYosys, Yosys and Z3 oracle when
each is measured as an external process?

This is not a solver race. It measures two different trust-transfer choices:

- GCC produces an 8,549-byte, source-bound artifact that a fresh process checks
  by exhaustive controller equivalence and complete plant-result recomputation;
- the formal route recompiles the same closed-loop model and reruns one
  incremental Z3 session, but does not emit an independently replayable proof
  artifact for a separate consumer.

Both paths reproduce four UNSAFE results at frames 4, 7, 15 and 15 plus two SAFE
results through frame 32.

## Frozen method

`scripts/benchmark-controller-mtbdd-process-resources.sh` uses `/usr/bin/time`
around three whole-process operations: GCC production, fresh GCC verification,
and the maintained formal oracle. It records wall time and maximum resident set
size. Trial count is explicit, bounded to 100, and never influences backend
selection.

The retained host used macOS 26.5.1 arm64, Rust 1.97.0, Yosys 0.67+post at
`b8e7da6f40ae8f552c116bf6c359b07c6533e159`, Z3 4.16.0, and SymbiYosys at
`fea6e467d067b3ea84b6b5ac08cd48beb59f0d42`.

## Result

Three trials give these medians:

| Process | Wall time | Peak RSS |
|---|---:|---:|
| GCC artifact production | 2.41 s | 17,350,656 bytes |
| GCC fresh verification | 0.96 s | 4,341,760 bytes |
| SymbiYosys oracle | 0.36 s | 29,392,896 bytes |

The formal oracle is 2.67 times faster than fresh GCC verification on this host.
Including GCC production makes the initial GCC transfer 9.36 times slower than
rerunning the oracle. This is a clear negative speed result and rules out a
runtime-performance claim for this fixture.

Fresh GCC verification uses 85.2% less peak resident memory than the formal
oracle and consumes the portable 8,549-byte artifact without Yosys, Z3 or the
producer's in-memory state. This demonstrates a concrete low-memory and
evidence-delivery advantage, not a novel algorithm.

The machine-readable trials are
`results/controller-mtbdd-process-resources-v1.csv`.

## Limits

Peak RSS semantics differ between BSD and GNU `time`, so values must not be
compared across operating systems as if they came from one population. The
script labels both backend and platform. It measures process envelopes, not
individual synthesis or solving phases. The plant is repository-authored, the
host is not independent, and the external route does not export a comparable
proof artifact. Hosted Linux, non-repository-authored plant, independent
acceptance and closest-system expert review remain open.
