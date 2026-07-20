# Equivalent-evidence Certifaiger comparison plan

Status: predeclared experiment plan. No result or novelty claim.

## Why this is the next gate

The identical-query SymbiYosys baseline is negative on runtime, but its formal
path emits no independently replayable certificate. That leaves GCC's strongest
claimed operational distinction, portable source-bound batch evidence, compared
against a tool with a different evidence contract.

Certifaiger is the appropriate established control. Its public implementation
checks AIGER witness circuits by reducing simulation, safety, and inductiveness
conditions to SAT. It can stream LRAT proofs to an external checker. The
[HWMCC 2025 rules](https://hwmcc.github.io/2025/) require bit-level SAFE
certificates to pass Certifaiger and UNSAFE traces to pass `aigsim`. The
[Certifaiger repository](https://github.com/Froleyks/certifaiger) documents the
witness format and independent checking obligations. This is current
competition practice, not a historical straw baseline.

## Exact scope

Use the existing public washing-controller and physical-plant family. Export six
single-property AIGER 1.9 models that preserve, byte-for-byte where applicable:

- controller and plant transition functions;
- controller and plant initial states;
- sensor/action wiring;
- each selected bad output; and
- the existing horizon of 32.

Encode the bound with a checked frame counter and an absorbing completed state,
so unbounded safety of the exported model is equivalent to the original bounded
query. Before benchmarking, an independent small-state replay must prove each
exported model agrees with the original query on SAFE/UNSAFE and shortest bad
frame.

The external path must use the pinned interface expected by HWMCC:

1. a certifying model checker produces one SAFE witness circuit or one UNSAFE
   AIGER trace per property;
2. Certifaiger checks every SAFE witness;
3. `aigsim` checks every UNSAFE trace; and
4. source, model, property, bound, tool revision, and certificate digests are
   retained in a canonical manifest.

The GCC path uses one proof-carrying controller MTBDD batch artifact and its
fresh independent verifier. Both paths must run in separate producer and
consumer processes without producer caches.

## Measurements

Record these separately for producer and consumer:

- wall time and peak RSS;
- total and per-property evidence bytes;
- model/export bytes that cross the trust boundary;
- checker executable and dependency footprint;
- number and type of SAT, QBF, or proof-checking obligations;
- exact answers and shortest bad frames;
- deterministic reproduction across two clean directories; and
- rejection of mutation, truncation, source drift, property substitution,
  member reordering, stale evidence, and output collision.

No timing observation may influence routing. All unsupported or malformed cases
fail closed.

## Advancement and falsification gates

The evidence-delivery distinction advances only if GCC preserves every answer
and trace while demonstrating at least one independently useful advantage over
the competition-standard certified path, such as smaller aggregate evidence,
lower fresh-checker resources, or one source-bound batch replacing repeated
per-property evidence.

The distinction is falsified for this regime if the Certifaiger plus `aigsim`
path provides equivalently bound, independently checkable evidence with no
material disadvantage in transfer size, checker resources, or integration. A
GCC win against an uncertified solver is irrelevant. A speed win caused by a
weaker bound, omitted source binding, missing SAFE proof, or unvalidated UNSAFE
trace is invalid.

## Pre-implementation gates

- Pin auditable source revisions and licenses for the producer, Certifaiger,
  AIGER utilities, SAT/QBF solvers, and optional LRAT checker.
- Build and run upstream tests in an isolated Linux container.
- Freeze the six exported model digests only after independent semantic replay.
- Add no GCC portfolio route until the complete external comparison and hostile
  controls pass on hosted Linux.

## Qualification finding

The first source audit pins Certifaiger 10.2.0 at commit
`3b8d9e9937234b5e064923bd00f20d3eb97ccc3f` from 6 July 2026. Its upstream
high-assurance Docker build selects CaDiCaL plus `lrat_isa`, but several CMake
dependencies still default to moving branches such as `main`, `master`,
`development`, or the AIGER development branch. Building the commit alone is
therefore not reproducible. The GCC qualification harness must override every
dependency with an immutable commit and retain the resolved revision manifest.

The first isolated build attempt also failed while retrieving the Alpine 3.22
base-image metadata from Docker Hub. That network failure is not a tool result.
No comparison run is accepted until both the base image and every source
dependency are content-addressed and the upstream tests pass.

The immutable qualification inputs are recorded in
`tools/certifaiger-qualification-v1.lock`. The qualification harness verifies
clean checkouts at those exact commits, disables container networking, and
passes every dependency as a local CMake source directory. This makes an
unexpected fetch or moving-branch resolution fail instead of silently changing
the comparison. Both qualification hosts use the Ubuntu 24.04 base with
digest `sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90`.
The hosted amd64 reconstruction passed from clean checkouts on 20 July 2026;
its complete image and binary provenance is retained under `results`.

The first offline arm64 compile reached all local dependency builds and then
failed because `lrat_isa` invokes `clang++` directly. This exposed an undeclared
toolchain dependency in the upstream container recipe. The local qualification
image now pins Ubuntu's Clang 18 package explicitly; the failed run is not
treated as a Certifaiger result.

The second offline compile found a further implicit `lrat_isa` dependency: its
fixed `-fuse-ld=lld` flag also requires LLVM's linker package. The qualification
image pins the matching LLD 18 package as well. This second failed run likewise
produced no pass marker and is not benchmark evidence.

The third offline compile built Certifaiger itself but `lrat_isa` could not link
`boost_iostreams`. The umbrella Boost headers package is insufficient on Ubuntu;
the qualification image now pins the matching Boost iostreams development
package explicitly. No partial binary from this run is accepted.

The fourth offline compile and install passed, including the `lrat_isa` build's
own checker test. Qualification also replays every upstream Certifaiger witness
fixture serially and requires exact agreement with `tests/expected-invalid`, so
the result does not depend on GNU Parallel being present in the container.

The final local arm64 qualification passed on 20 July 2026. The toolchain image
ID is `sha256:da1dd2f2e859a343cdf3f97500a23368dd4b69fe20ddca4a76b91f9a290800c5`.
All nine valid upstream witnesses were accepted and the single intentionally
invalid witness was rejected. The upstream-test log digest is
`03792c83051ac80979650918190fb5c6b96eabf2f98340c78956d07ff20e0257` and the
build-log digest is
`3177300821a1e3e209330689fb4bc9926e7d40e78b13c779728b780a22949736`.
These hashes qualify the toolchain only; they are not comparison measurements.

The producer is rIC3 1.5.2 at commit
`7149d568785b039134f0b2baa58358c8af63e70d`, with all 12 recursive submodule
commits, its Cargo lockfile, and a checksum-locked vendor tree recorded in the
qualification lock. It passed its test suite and release build without network
access under Rust 1.97.0. The qualified arm64 image ID is
`sha256:800edccd857d5a514f983b2292d29bd0db8e56eff6efcceccc7fc6a8ad92d92f` and
the rIC3 binary digest is
`3bddece2e0beeebb3b116158968f51ddf4345a8a346ba77679538729d30c11bb`.
The independently rebuilt amd64 image ID is
`sha256:8afe7af40e7d5cdfb6fcb2d3f5c9f62f671006544d363d469f5205887a5198b8`
and its rIC3 binary digest is
`12a1351c482b448e9eb8c9522ff2de4f5c1eea22a900ef7fedaffc7bd0492a71`.

An end-to-end fixture then exercised both evidence classes with independently
built consumers. rIC3 proved the infusion-pump safe controller and emitted a
69-byte SAFE witness with digest
`ff90aa1367dc428f85045ca519febdee4371ab63d729acfbed68f0b1f0f4f48e`;
Certifaiger accepted it after CaDiCaL and `lrat_isa` checked all obligations.
rIC3 also found the door-interlock regression at depth 1 and emitted a 15-byte
UNSAFE witness with digest
`b9216a7ad88c824155e6b7a865e3575a3b608e928c4963dd2efeb02c985d12cc`;
`aigsim -c -m` replayed it successfully. These are qualification fixtures, not
the predeclared six-property comparison.

## Frozen equivalent models and evidence

The six bounded-equivalent exports are frozen under
`corpus/rtl/wmcontroller/certified-baseline-v1`. Each combines the original
controller and plant latches with four external disturbance inputs and a
six-bit checked frame counter. Frames 0 through 32 preserve the selected bad
output; frame 33 is absorbing and suppresses bad. A separate integration-test
parser explored the emitted transition systems and reproduced the source
results and shortest bad frames: UNSAFE at 4, 7, 15, and 15, then two SAFE
properties whose reachable layers converge after completion.

All six models are 2,488 bytes. Their canonical digests and source bindings are
in `manifest-v1.txt`. The qualified rIC3 producer returned SAT, SAT, SAT, SAT,
UNSAT, and UNSAT. The four traces were replayed with `aigsim -c -m`; the two
SAFE witnesses were accepted by Certifaiger with CaDiCaL and `lrat_isa`. Total
external evidence is 5,130 bytes: 51, 66, 106, 106, 2,412, and 2,389 bytes.
The retained companion manifest binds those member sizes and digests to the
frozen model manifest, qualification lock, producer binary, and consumer tree.
This closes semantic equivalence and both evidence-class qualification gates.
The retained manifests now make clean-directory and cross-platform certificate
identity directly auditable rather than reducing it to a Boolean result flag.

A single-engine IC3 negative control produced valid but nonminimal traces for
properties 13 and 14 at frame 29 instead of the independently established
shortest frame 15. It also accepted a property-12 trace as valid property-11
evidence because reachability witnesses do not certify minimality. An initial
rIC3 portfolio probe produced frame 15, but a clean rerun produced frame 25, so
first-answer portfolio evidence is also unsuitable for an exact shortest-trace
contract.

The accepted baseline uses the same static minimality-aware race for every
formula. BMC and IC3 start together; depth-ordered BMC SAT is accepted, IC3 SAT
is provisional until BMC finishes, and Certifaiger-compatible IC3 UNSAT is
accepted immediately. The losing process is cancelled. No answer, timing, or
formula-specific calibration selects the engine. The consumer additionally
checks each UNSAFE witness frame against the independently replayed frozen
manifest before invoking `aigsim`.

## Local arm64 comparison finding

Three clean network-disabled trials use the same pinned Linux host, Rust 1.97
base, `runlim` 2.0.0rc14 sampler, six models, and exact answers. The
competition-standard path has median production time 0.21 seconds, consumer
time 0.30 seconds, producer space 24 MB, consumer space 127 MB, and 5,130 bytes
of evidence. GCC has median batch production time 1.38 seconds, verification
time 0.49 seconds, producer space 13 MB, verifier space 7 MB, and 251,221 bytes
of evidence.

This is a strong negative result for GCC transfer size and runtime in this
regime. Standard evidence is 48.97 times smaller, production is 6.57 times
faster, and checking is 1.63 times faster. The surviving GCC advantages are
45.8% lower producer space and a fresh verifier memory profile 18.14 times
smaller at the sampler's 1 MB resolution. GCC's unstripped 78,167,632-byte
executable is also much larger
than the 7,353,840-byte rIC3 binary or 10,280,960-byte Certifaiger tool
directory. No speed, size, or packaging advantage may be claimed from this
experiment. Hosted amd64 replication is reported separately below.

The local hostile suite accepts the unchanged six-member package and rejects
witness mutation, truncation, cross-property substitution, member reordering,
stale evidence, output collision, and model source drift. The frame-binding
control is material: without it, `aigsim` correctly accepts some later valid
traces that do not preserve GCC's shortest-trace result.

## Hosted amd64 replication finding

The exact-head [GitHub-hosted run 29741875233](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29741875233)
rebuilt every pinned producer and consumer dependency, passed upstream tests
without container networking, ran
three native amd64 trials, and repeated all seven hostile controls. The standard
path has median production time 0.17 seconds, consumer time 0.34 seconds,
producer space 25 MB, consumer space 115 MB, and 5,130 bytes of evidence. GCC
has median creation time 2.21 seconds, verification time 0.75 seconds, producer
space 15 MB, verifier space 7 MB, and 251,221 bytes of evidence.

The standard path is therefore 48.97 times smaller, 13.00 times faster to
produce, 2.21 times faster to check, and its combined producer and consumer
binary footprint is 3.96 times smaller than the GCC executable. GCC uses 40.0%
less producer space and 16.43 times less verifier space. The low-memory direction
replicates, but the runtime, transfer-size, and packaging results remain strong
negatives.

All six external witness sizes and SHA-256 digests are identical between the
arm64 and amd64 manifests. GCC's batch proof digest is also identical between
an exact-head macOS arm64 creation and the hosted Linux amd64 creation:
`b80aff5de88bfe7e42dbe1b531ef9b48046a3c129c7b5dce243987423dfe655a`.
Both independently verified the same 251,221-byte artifact. This closes hosted
replication, hostile-control, and cross-platform deterministic-evidence gates
for this bounded corpus.

The durable records are the
[standard-path measurements](../results/certified-evidence-equivalent-amd64-v1.csv),
[standard-path manifest](../results/certified-evidence-equivalent-amd64-v1.manifest-v1.txt),
[GCC measurements](../results/gcc-proof-equivalent-amd64-v1.csv),
[GCC manifest](../results/gcc-proof-equivalent-amd64-v1.manifest-v1.txt),
[hostile controls](../results/certified-evidence-hostile-amd64-v1.csv),
[cross-platform proof record](../results/gcc-proof-cross-platform-v1.txt), and
[hosted provenance](../results/certified-evidence-hosted-amd64-v1-provenance.txt).
CI checks their agreement with `scripts/check-certified-evidence-retained-v1.sh`.

The predeclared lower-resource criterion advances only a narrow constrained
memory verifier profile. It does not support a general speed, size, packaging,
or solver-performance claim, and it does not establish novelty. No automatic
portfolio route is added from this result. A production route would first need
an explicit resource policy and external acceptance on a genuinely constrained
firmware or robotics workflow.
