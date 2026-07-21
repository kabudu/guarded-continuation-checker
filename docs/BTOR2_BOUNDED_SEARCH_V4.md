# BTOR2 bounded search certificate v4

Status: experimental implementation with local and hosted public-design
validation.

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

## Retained local result

The implementation passes the predeclared local gates. V4 binds one or more
ordered constraint nodes and one through eight ordered one-bit inputs. Its
producer and verifier independently enumerate the full valuation space, admit
only valuations satisfying every current-frame constraint, preserve distinct
terminal valuations, and retain empty suffix layers after an assumption dead
end. Static work charging still uses all valuations.

A separately implemented exhaustive trace oracle agrees across constrained
SAFE and UNSAFE cases, state-only and input-dependent properties, mutually
exclusive inputs, state-dependent assumptions, one- and two-input models, and
dead-end traces. Constraint omission, rebinding, reordering, count drift,
downgrade, inadmissible witness valuations, false successors, truncation, and
no-clobber controls fail closed.

The pinned Roa Logic PLIC wrapper now emits two actual BTOR2 constraints over
five semantic inputs. Both properties verify SAFE through horizon 16, with
1,138 aggregate reachable-state occurrences and a 121,639-byte predicate-set
artifact. Two model builds and two evidence builds are byte-identical. Yosys
plus Z3 independently checks the same assumptions and assertions through step
16. The retained v1, Caliptra v2, and PLIC v3 result files remain byte-identical.
The public result is recorded in
[`roalogic-plic-gateway-constrained-acceptance-v1.csv`](../results/roalogic-plic-gateway-constrained-acceptance-v1.csv).

[Hosted amd64 run
29872388711](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29872388711)
reproduces the constrained PLIC model and evidence using the pinned Yosys
source build, checks both properties with maintained Yosys plus Z3, preserves
the retained v1 through v3 evidence, and passes the complete Linux and
downstream API suites. This closes the predeclared v4 gates. It does not change
the explicit non-novelty claim or make the wider platform production-ready.
