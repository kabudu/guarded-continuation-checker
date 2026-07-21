# BTOR2 bounded search certificate v2

Status: experimental additive format. V1 remains supported unchanged.

## Capability

Search certificate v2 extends exact bounded reachability to a model whose bad
property depends on the current one-bit semantic input. This occurs in ordinary
Yosys lowering of asynchronous reset logic, including the pinned Caliptra
watchdog experiment.

The transition inputs in `witness` select the edges used to reach the reported
bad frame. The separate `terminal_input` selects the input valuation used to
evaluate the bad property at that frame. It is `0` or `1` for an UNSAFE v2
certificate and `none` for SAFE. This distinction is necessary because the
input selecting the final transition and the input visible at the resulting
frame are not the same semantic object.

For SAFE, the certificate retains the complete canonical reachable-state layer
at every frame through the requested horizon. The independent verifier checks
the bad property under both input values for every retained state and rebuilds
the complete two-input successor set between layers. For UNSAFE, it replays all
transition inputs and then evaluates the bad property with the bound terminal
input. No input value is inferred or omitted.

## Compatibility

State-only properties continue to produce v1. Their encoding has no
`terminal_input` line and the retained v1 cohort remains byte-identical. The
decoder accepts v1 and v2, but verification rejects:

- v1 evidence for an input-dependent bad property;
- a terminal input in v1;
- v2 evidence for a state-only bad property;
- missing or invalid v2 terminal input for UNSAFE;
- any terminal input for SAFE;
- terminal-input mutation that no longer activates the bad property; and
- version downgrades or incomplete v2 text.

The public `search_certificate_version` capability reports the latest producer
version, 2. Individual create and verify responses report the artifact's actual
version. Existing v1 artifacts are never reported as v2.

## Resource and claim boundary

V2 retains v1's static limits: one one-bit semantic input, no constraints,
horizon at most 256, at most 65,536 states per layer, 262,144 states total,
20,000,000 estimated node steps, and a 16 MiB certificate. Refusal returns no
logical answer or partial artifact.

This is an exact explicit-state certificate extension, not recurrence
acceleration or an algorithmic novelty claim. It closes a standard Yosys
interoperability gap and makes reset-dependent bounded properties checkable
without changing v1 semantics.
