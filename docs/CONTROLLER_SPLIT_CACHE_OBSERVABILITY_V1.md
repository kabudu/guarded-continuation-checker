# Controller split cache observability v1

Status: stable candidate additive CLI and Rust API. Not a production release.

This contract exposes integrity-preserving, process-local semantic replay reuse
for governed split verification. It exists to answer whether repeated verified
batches avoid duplicate replay without allowing a cache hit to bypass policy,
source/model, artifact, obligation, or resource checks.

## Integrity boundary

The verifier always completes both passes before consulting the cache. For each
batch it rereads the manifest and plant-result artifact, reloads the bound
source/model snapshot, checks that the inputs did not change after preflight,
recomputes the resource assessment, and verifies the result SHA-256.

Only then does it look for an entry keyed by the complete manifest value,
source/model snapshot, resource assessment, and plant-result SHA-256. A hit
reuses the already checked semantic replay summary. A miss performs exact
semantic replay and inserts that summary. Entries live only for the current
process invocation and are bounded by the discovered batch limit.

This is not a persistent cache, an answer cache across calls, or a permission to
trust paths, timestamps, producer metadata, or partial digests. Existing v1
commands retain their previous uncached execution path.

## Discovery

```text
guarded-continuation-checker controller-split-cache-observability-cli-version
```

The response contains the three preceding resource, phase, and allocation
capability lines followed by one strict cache line. The final line declares:

- semantic replay scope;
- the complete cache-key vocabulary;
- lookup, hit, miss, and entry counters;
- mandatory integrity preflight;
- checked fail-closed counter behavior; and
- no timing calibration, partial failure metrics, or refusal result.

Consumers must reject missing, reordered, additional, non-canonical, or changed
fields.

## Verification

```text
guarded-continuation-checker \
  verify-bound-plant-result-set-with-resources-cache-observed-v1 \
  INPUT.controller-evidence POLICY.txt \
  MANIFEST.txt INPUT.plant-results \
  [MANIFEST.txt INPUT.plant-results ...]
```

After the existing result, phase, and allocation rows, success emits:

```text
controller-split-cache-observability status=MEASURED cli_version=1 scope=semantic-replay key=manifest-snapshot,resource-assessment,result-sha256 lookups=N hits=N misses=N entries=N integrity_preflight=required overflow=none timing_calibration=none
```

The typed client requires `lookups = batches`, `hits + misses = lookups`, and
`entries = misses`. All arithmetic is checked. Empty, overflowing, hostile, or
non-canonical values fail with the stable response failure class.

Producer-side refusal, invalid input, semantic failure, allocation-counter
overflow, or cache-counter overflow emits no partial cache row. The typed client
returns no typed result for those failures, timeouts, output failures, or
hostile helper responses. Known resource refusals retain their typed reason and
process metrics.

## Evidence and limits

The retained acceptance runs ordinary misses plus a duplicate-batch probe that
must report one miss and one hit while preserving exact answers. Its canonical
CSV remains timing-free and unchanged because cache counts depend on the batch
composition being exercised.

This closes observability of the current process-local cache mechanism. It does
not establish a persistent service cache, cross-process speed-up, safe reuse
after a release change, peak memory, or product superiority.
