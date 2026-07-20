# Proof-carrying controller MTBDD CLI v1

Status: experimental fast-verification interface. It is separate from the
compact controller MTBDD CLI so existing discovery and output contracts do not
change silently.

## Discovery

```sh
guarded-continuation-checker controller-proof-mtbdd-cli-version
```

The canonical response identifies CLI, MTBDD, equivalence-proof, plant
artifact, and manifest versions. It also reports every static byte and model
limit, `verification=unsat-miter`, `exhaustive_replay=no`, and
`unsupported=fail-closed`. Unknown, missing, reordered, or changed fields are
incompatible.

## Create and verify

```sh
guarded-continuation-checker certify-controller-proof-mtbdd-plant-batch \
  corpus/rtl/wmcontroller/physical-plant-batch-v1.txt \
  physical-plant.proof-mtbdd-plant

guarded-continuation-checker verify-controller-proof-mtbdd-plant-batch \
  corpus/rtl/wmcontroller/physical-plant-batch-v1.txt \
  physical-plant.proof-mtbdd-plant
```

Both commands load the existing canonical manifest v1, bind its ordered source
digests and queries, check the source-bound SAT miter proof, and independently
recompute every plant result. Creation uses create-new semantics. Mutation,
truncation, manifest drift, source drift, member reordering, unsupported models,
and resource-limit violations fail closed.

`assignments_checked=0` is intentional. The proof represents the complete
state/input scope but the verifier does not replay individual assignments.

## Rust process API

`ControllerProofMtbddTool` discovers the complete contract, executes without a
shell under `ExecutionPolicy`, and returns typed ordered results plus invocation
metrics. It rejects noncanonical output, incompatible versions, changed proof
semantics, count disagreement, member reordering, and responses outside the
discovered limits.

## Compatibility and claim boundary

The compact `controller-mtbdd-cli-version` contract remains byte-for-byte
unchanged. The new commands and `GCCMPF01` artifact begin compatibility history
at version 1. SAT miters and UNSAT proof checking are established techniques;
this interface is product integration, not a novelty claim. Hosted
whole-process evidence and independent acceptance remain required before the
proof profile can enter the default portfolio.

## Simulated external acceptance

`scripts/run-controller-proof-mtbdd-self-service-acceptance.sh` exercises the
public six-property physical-plant manifest through fresh producer and verifier
processes. The retained
`results/controller-proof-mtbdd-self-service-acceptance-v1.csv` records all four
expected shortest UNSAFE frames, both SAFE results, proof verification,
manifest-drift rejection, mutation rejection, and no-clobber rejection. This is
repository-run simulated acceptance, not independent partner evidence.
