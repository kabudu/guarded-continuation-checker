# Public controller with stateful physical plant v1

Status: experimental mechanism evidence. The fixed experiment passes its exact
answer, evidence-transfer, checked in-process, maintained external symbolic
session, and local process-resource gates. Hosted Linux resource replication
remains open.

## Predeclared question

Can one source-bound controller MTBDD support a stateful physical plant family
without losing exact answers, while retaining the evidence-transfer advantage
after comparison with checked in-process reuse?

The controller MTBDD gates remain unchanged: six controller-state bits, eleven
sensed inputs, four selected action outputs, at most 512 decision nodes, and
complete checking of all 131,072 controller assignments. No admission limit or
variable order was changed after observing the plant results.

## Plant and properties

Two complementary repository-authored plants are retained. The deterministic
full-action process model observes fill, heat, wash, rinse, spin, fault, and
water-intake outputs. Its five state bits record progress through the physical
cycle, and three expected-SAFE properties exclude dry heating, spin during
water intake, and water intake outside fill or rinse. Supporting seven actions
required an explicitly reviewed additive increase of the MTBDD observed-output
limit from four to eight; the decision-node, terminal, assignment, and byte
limits did not change. The resulting controller MTBDD still has 254 decision
nodes, 189 terminals, and 131,072 checked assignments.

The repository-authored plant has six state bits for water level, cycle
progress, door state, imbalance, and motor failure. Three unconstrained event
inputs update the stored disturbance state. This yields a 64-state physical
plant and a statically bounded 4,096-state controller-product space.

Six ordered properties cover water intake with an open door, overfill, spin
under imbalance, spin after motor failure, actuation during controller fault,
and conflicting fill plus spin commands. They yield four UNSAFE and two SAFE
answers through horizon 32. Every shortest bad frame and complete result agrees
with direct AIGER controller evaluation.

The three full-action process properties also agree exactly with direct AIGER
controller evaluation through horizon 64. They are a breadth control for the
expanded action boundary, while the disturbance plant supplies both answer
classes and the three-way cost comparison.

## Three-way retained result

Three interleaved release-mode trials produced these medians:

| Path | Boundary bytes | Consumer or checked-reuse time |
|---|---:|---:|
| Repeated complete evidence | 39,879 | 3,370,832,666 ns |
| One shared checked artifact | 8,549 | 984,596,292 ns |
| Checked in-process MTBDD reuse | no serialized boundary | 996,516,875 ns |

The shared artifact is 78.6% smaller and checks 70.8% faster than repeated
complete evidence. Its retained checking-time ratio against checked in-process
reuse is 0.988, which is practical parity rather than a claimed speed win. The
important measured result is that canonical decoding, source and query binding,
result comparison, and integrity checking did not create a material penalty in
this small run.

Reproduce with:

```sh
cargo run --release --locked \
  --example public_washing_controller_physical_plant_benchmark
```

The machine-readable reference is
`results/public-washing-controller-physical-plant-v1.csv`.

## Interpretation and open gates

This improves the evidence from constant-sensor mechanism tests to a stateful
product-shaped physical environment with nondeterministic disturbances. It is
still repository-authored, deliberately small, and not calibrated against a
real appliance. It therefore does not count as a second public product or an
independent acceptance result.

Checked in-process reuse is a stronger runtime control than repeated evidence,
but it is still GCC's MTBDD implementation. A pinned SymbiYosys session with
maintained Yosys and Z3 now compiles the same closed loop once, checks all six
assertions together, reproduces bad frames 4, 7, 15, and 15, and reaches frame
32 without either expected-SAFE assertion failing. Reproduce with:

```sh
scripts/test-public-washing-controller-physical-oracle.sh /path/to/sby.py
```

The retained agreement rows are in
`results/public-washing-controller-physical-oracle-v1.csv`. A subsequent
[whole-process resource baseline](CONTROLLER_MTBDD_PROCESS_RESOURCES_V1.md)
finds that the formal oracle is 2.67 times faster than fresh GCC verification
on the retained arm64 host, while GCC verification uses 85.2% less peak RSS and
consumes a portable 8,549-byte artifact. This is not a speed or novelty claim.
Hosted Linux replication remains open.

## Maintained single-session oracle

Pinned SymbiYosys, maintained Yosys, and Z3 now compile the exact controller,
plant, wiring, initial state, and six assertions once. One incremental
`yosys-smtbmc --keep-going` session reproduces the four UNSAFE shortest frames
at 4, 7, 15, and 15, while the fault-actuation and conflicting-action
assertions remain valid through frame 32. These results exactly match GCC.

This closes the maintained external batch-answer gate for this fixture. The
process-resource follow-up reports the different whole-command boundaries
explicitly rather than presenting them as equivalent solver phases.

## Seven-action complete-cycle follow-up

A second repository-authored plant models fill, heat, wash, rinse, and spin
completion as stored physical state. It observes seven controller actions, so
the predeclared MTBDD output boundary increases from four to eight. The existing
131,072-assignment, 512-node, 1,024-terminal, and one-megabyte caps remain
unchanged.

The resulting public-controller MTBDD has 254 nodes and 189 terminals. Dry
heating, spinning with water intake, and uncommanded water intake all remain
SAFE through horizon 64 and exactly equal direct AIGER evaluation. This proves
the representation supports the complete cycle action surface, but the plant
remains repository-authored and uncalibrated.
