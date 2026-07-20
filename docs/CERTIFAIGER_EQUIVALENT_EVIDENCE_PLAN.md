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
the comparison. The first cached base is the arm64 Ubuntu 24.04 image with
digest `sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90`.
An independently pinned amd64 image remains required for hosted Linux evidence.

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
