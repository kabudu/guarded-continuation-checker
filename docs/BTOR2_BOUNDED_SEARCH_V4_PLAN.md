# BTOR2 bounded search certificate v4 plan

Status: predeclared experiment; no result is claimed.

## Capability gap

Bounded search v3 preserves complete valuations for up to eight one-bit inputs,
but refuses every BTOR2 model containing a `constraint`. That excludes ordinary
environment contracts such as legal reset sequences, mutually exclusive bus
events, interrupt protocol rules, and actuator interlocks. Silently dropping a
constraint would enlarge the trace language and could change either answer.

## Candidate semantics

Certificate v4 will admit one to eight one-bit inputs and one or more canonical
BTOR2 constraints. It will bind the complete ordered input and constraint-node
lists. At each frame, only valuations satisfying every constraint in the
current state are admissible:

- an admissible valuation may activate the selected bad property;
- an admissible nonterminal valuation contributes its exact successor;
- an inadmissible valuation contributes neither a bad observation nor a
  successor; and
- a layer after an assumption dead end is canonically empty, with every later
  layer also empty.

UNSAFE evidence must retain every complete transition valuation and a distinct
admissible terminal valuation. SAFE evidence must retain the complete ordered
reachable-state layer at every frame, including empty suffix layers. The
verifier will independently evaluate all constraints and reconstruct the exact
admissible successor relation from the source model.

## Compatibility and governance

V1, v2, and v3 production and encoding remain unchanged. V4 is selected only
when at least one constraint exists. The existing horizon, state, node-step,
artifact, input-width, and input-count limits remain in force. Work accounting
uses the full valuation count, not the number that happens to be admissible, so
a restrictive assumption cannot evade preflight governance.

An additive capability command will disclose the exact constraint contract.
Constraint omission, reordering, duplicated identifiers, version downgrade,
inadmissible witness valuations, false dead-end claims, truncation, oversized
inputs, and resource exhaustion must fail closed without a logical answer or
partial artifact.

## Predeclared gates

The cycle passes only if:

1. retained v1, Caliptra v2, and PLIC v3 artifacts remain byte-identical;
2. v4 produces and independently verifies constrained SAFE and UNSAFE cases;
3. generated models cover state-only, input-dependent, mutually exclusive,
   state-dependent, and assumption-dead-end constraints;
4. every result and earliest bad frame agrees with a separately implemented
   exhaustive trace oracle;
5. every constraint-binding, valuation, layer, downgrade, truncation, and
   resource hostile control fails closed;
6. a pinned public embedded RTL workflow emits real BTOR2 constraints and
   agrees with maintained Yosys plus Z3 on accepted horizons; and
7. the full hosted Linux suite and downstream API matrix pass.

## Claim boundary

Assumption-constrained explicit-state search is established model-checking
practice. V4 can close a practical workflow and semantic-integrity gap, but it
cannot establish algorithmic novelty. Any novelty candidate remains in later
proof compression or reusable component evidence built above this exact
fallback.
