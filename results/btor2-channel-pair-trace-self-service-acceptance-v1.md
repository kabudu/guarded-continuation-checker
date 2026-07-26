# BTOR2 channel-pair trace self-service acceptance v1

This local operator-style run exercises the fixed OpenTitan PWM workflow
through the public executable. It is simulated acceptance evidence, not
independent partner validation.

## Fixed inputs

- Model: `symbolic-class-6.btor2`
- Query manifest: `pair-trace-queries-v1.txt`
- Resource policy: `trace-policy-v1.txt`
- CLI and artifact version: 1

## Retained result

| Logical queries | Proof members | Reused queries | Evidence | Artifact |
|---:|---:|---:|---:|---:|
| 12 | 4 | 8 | 4 B | 912 B |

The artifact SHA-256 is
`aac88db71d0b536a7b2c4aed4c3f37315e29a15affbdb1f260db94136e994e51`.
The projected-work admission token is 6.

Certification creates the artifact once and verifies all twelve results before
publication. A separate verify invocation decodes the saved bytes, rederives
the structural admission from the source, checks every member and replays every
UNSAFE target. A second clean output is byte-identical.

The acceptance test also retains these negative controls:

- an existing output is preserved and certification refuses to overwrite it;
- relation drift between the manifest and artifact returns no answer;
- one changed artifact byte returns no answer;
- identical left and right endpoints are rejected before production; and
- malformed, oversized, linked or non-regular inputs follow the shared bounded
  file boundary.

Cross-platform artifact identity, whole-process resources, maintained-tool
comparison, tagged compatibility and an independent operator remain open.
