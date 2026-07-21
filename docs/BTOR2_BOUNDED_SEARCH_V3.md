# BTOR2 bounded search certificate v3

Status: experimental implementation with local public-design validation. Hosted
reproduction remains open.

## Problem exposed by public RTL

The pinned Roa Logic PLIC gateway candidate has five semantic one-bit inputs:
reset, interrupt source, edge/level mode, claim, and completion. Pinned Yosys
lowers the unmodified gateway plus a property wrapper to a valid 97-node BTOR2
model with seven states and two bad properties. Existing bounded search v2
refuses it because v2 admits exactly one one-bit input.

This is not merely a parser gap. Exact SAFE evidence must quantify every input
valuation at every reachable state, while an UNSAFE trace must preserve the
complete valuation selecting each transition and a distinct terminal-frame
valuation. Treating several inputs as one Boolean or omitting the terminal
valuation would lose semantics.

## Candidate format

Certificate v3 will admit between two and eight semantic one-bit inputs in
canonical BTOR2 node order. It will record:

- the complete ordered input-node list;
- one packed unsigned valuation per witness transition;
- a separate packed terminal valuation for UNSAFE;
- complete canonical reachable-state layers for SAFE; and
- the existing source, property, horizon, result, and resource bindings.

Bit `i` of a packed valuation binds input node `inputs[i]`. Values with bits
outside the declared input count are noncanonical and rejected. SAFE evidence
has no terminal valuation. The verifier independently enumerates every one of
the `2^inputs` valuations for bad-property checks and successor reconstruction.

The additive `btor2-search-v3-capabilities` command reports the exact input,
horizon, state, work, artifact, valuation-order, refusal, and unsupported-shape
contract without changing the retained `btor2-cli-version` response.

## Compatibility

V1 and v2 encoding, decoding, production, and verification remain unchanged.
V1 remains the producer for one-input state-only bad properties. V2 remains the
producer for one-input current-dependent bad properties. V3 is selected only
for a model with two or more semantic inputs. Cross-version reinterpretation,
input reordering, downgrade, missing valuation, and high-bit mutation must fail
closed.

## Static resource gate

The experiment retains the existing horizon, layer, total-state, node-step,
and certificate-byte caps. Estimated work must multiply by the exact valuation
count before search or verification. At most eight one-bit inputs are admitted.
Constraints, wider inputs, overflow, and any bound violation return no logical
answer or partial artifact.

## Predeclared gates

The cycle passes only if:

1. all retained v1 artifacts remain byte-identical;
2. all retained Caliptra v2 artifacts remain byte-identical;
3. v3 produces and independently verifies both SAFE and UNSAFE multi-input
   cases;
4. complete valuation enumeration agrees with an independent brute-force
   oracle on a generated small-model cohort;
5. input omission, reordering, valuation mutation, terminal mutation,
   downgrade, truncation, oversized input count, and work exhaustion fail
   closed;
6. the PLIC gateway probe moves from the predeclared multi-input refusal to
   exact answers without a PLIC-specific route; and
7. the full hosted Linux suite and downstream API matrix pass.

## Claim boundary

Multi-input explicit-state search is established model-checking practice. V3
closes a practical public-RTL interoperability gap and provides a trusted
fallback for later proof compression. It is not an algorithmic novelty claim.

## Retained local result

The implementation passes all predeclared local gates. It supports two through
eight ordered one-bit inputs, independently reconstructs every complete SAFE
successor layer, and replays packed UNSAFE transition and terminal valuations.
Generated parity models for every admitted input count agree with their
closed-form exhaustive result for both answer classes. Nine inputs, high
valuation bits, reordered inputs, missing terminal values, truncation, forced
downgrade, and resource exhaustion fail closed.

The pinned Roa Logic PLIC gateway now verifies both wrapper properties as SAFE
at horizons 0, 4, 8, and 16. At horizon 16 the two certificates cover 1,138
logical reachable-state occurrences in 121,491 bytes. Horizon 64 refuses at the
static node-step limit without an answer. Yosys plus Z3 independently checks
both assertions through step 16. Two model builds and two evidence builds are
byte-identical, and four hostile workflow controls fail closed.

Retained v1 search artifacts and Caliptra v2 artifacts remain byte-identical.
The public PLIC result is recorded in
[`roalogic-plic-gateway-acceptance-v1.csv`](../results/roalogic-plic-gateway-acceptance-v1.csv).

Models containing constraints use additive
[certificate v4](BTOR2_BOUNDED_SEARCH_V4.md). Constraint-free multi-input
models continue to produce byte-identical v3 evidence.
