# Research findings and limitations

## Core construction

For a variable order, the compiler substitutes each Boolean choice into every
current residual CNF. Residuals are sorted, deduplicated, and merged only when
they are exactly equal. This is a Myhill–Nerode-style behavioral quotient of
assignment prefixes. Terminal representatives recover witnesses.

## Positive results

- Banded synthetic formulas retained approximately 9–13 peak classes through
  100 variables.
- On ten unseen admitted 100-variable formulas, 5,000 width-three assumption
  queries averaged 1.61 microseconds versus 3.17 microseconds for persistent
  Varisat with model extraction: a 1.97x per-query speedup.
- The measured compilation break-even averaged roughly 10,216 queries.
- Exact insertion repair averaged a 1.32x speedup over full recompilation in the
  corrected unseen corpus.
- The modular DIMACS example achieved a 3.10x query speedup, a 260-query
  break-even, and a 1,106-byte transition-only artifact.

## Negative results

- Random residual spaces grew rapidly: one 40-variable planted case reached
  284,617 peak states and took about 40.5 seconds.
- Automatic policies using maximum frontier, full frontier profile, affected
  suffix, assumption width, query volume, and declared stability all failed to
  win reliably on unseen mixed workloads.
- All 40 bundled SATLIB instances failed the conservative 16-bit gate.
- Query advantage vanished as assumptions approached complete assignments.

## Temporal repeated-transition experiment

A controlled bounded-width model-checking family holds a Boolean state constant
through a sequence of local equality transitions. Its exact continuation
vocabulary is `2^width + 1`: one class per state plus contradiction. The class
count is independent of horizon.

The first representation materialized and replayed the same transition layers.
Although its peak class count stayed bounded, it was slower than persistent
Varisat at most phase points because query execution still scanned states at every
time step. This separates bounded continuation width from an efficient artifact
representation.

An exact repeated-transition kernel stores the invariant once, detects conflicting
observations by state-bit identity, and expands the retained state to recover the
complete temporal witness. Across 17 admitted points (widths 2--10 and horizons
10--10,000 where measured), it agreed with Varisat and all returned witnesses were
valid. Per-query speedup over persistent Varisat ranged from 1.86x to 89.91x. At
width 6 and horizon 10,000 (60,006 variables and 120,000 clauses), the dense
quotient achieved 0.095x while the kernel achieved 89.91x.

This is a strong result for a transparent equality-transition subclass, not for
arbitrary bounded model checking. The kernel is derived from the known repeated
transition invariant; the next test must infer reusable kernels for a broader,
predeclared transition vocabulary without per-formula calibration.

## CNF-recognized transition vocabulary

The follow-up removes the need to identify the equality kernel manually. Given
only layered CNF, a recognizer reconstructs each output bit's deterministic local
truth table and matches it against a fixed vocabulary: copy, negation,
permutation, pairwise XOR, and `(a AND b) XOR c`. It then verifies that the
normalized clause template is identical at every time step. There is no
trial-solving, learned selector, or per-formula timing calibration.

For admitted formulas, the kernel stores the state transition over `2^width`
states and logarithmic jump tables. Partial observations are checked by jumping
from each candidate initial state to the observed times. Once a candidate is
found, the full temporal witness is reconstructed by replaying the recognized
local rules.

The predeclared phase grid contained 45 cases across widths 4, 6, and 8 and
horizons 10, 100, and 1,000. All cases agreed with persistent Varisat and all
returned witnesses were valid. Query speedup ranged from 1.44x to 11,476.84x.
A separate 30-case holdout used widths 5 and 7, horizons 37, 333, and 2,000, and
a different query seed; it also had complete agreement and valid witnesses, with
speedups from 2.31x to 3,795.02x.

The largest gains occur for XOR and circuit encodings, where generic clause-level
reasoning repeatedly rediscovers a compact deterministic transition. These are
specialized finite-state systems with width at most eight, not arbitrary SAT or
arbitrary model checking. The next generalization boundary is exact recognition
of compositions whose local rules are not individually in the fixed vocabulary.

## Exact recognition beyond named rules

The next experiment recognizes the entire repeated one-step relation directly.
After one-pass normalization verifies identical CNF at every time step, the
recognizer enumerates all current and next states. It admits the formula only if
every current state has exactly one satisfying successor. This accepts exact
compositions outside the named vocabulary while rejecting incomplete,
nondeterministic, or changing transition relations.

Four composed families were predeclared: three-input majority, a three-input
multiplexer, `(a XOR b) AND NOT c`, and `(a XOR b) XOR (c AND d)`. The fixed-rule
recognizer rejects these families; the exact relation recognizer accepts them.

The 36-case phase grid used widths 4, 6, and 8 and horizons 10, 100, and 1,000.
All cases agreed with persistent Varisat and returned valid witnesses. Measured
query speedup ranged from 30.68x to 7,808.06x. A separate 24-case holdout used
widths 5 and 7, horizons 37, 333, and 2,000, and a different query seed. It also
had complete agreement and valid witnesses, with speedups from 119.45x to
4,299.87x.

This broadens the result from a rule vocabulary to arbitrary deterministic
repeated transitions within a very small state width. Recognition is exhaustive:
its worst-case work is proportional to `2^(2*width)` times the one-step clause
count. It therefore does not remove the exponential dependence on width and does
not imply a general SAT improvement. Its plausible application is repeated-query
model checking for compact deterministic controllers and protocols.

## Local output-cone recovery

The follow-up avoids enumerating current/next state pairs when the repeated CNF
decomposes by output. Each clause must contain exactly one next-frame variable.
Clauses are grouped by that output, their current-frame dependencies are
discovered directly, and a total deterministic truth table is checked using only
those dependencies. Cross-output clauses and non-functions are rejected.

For output dependency bound `k`, semantic recovery takes approximately
`sum(output 2^k)` truth-table checks instead of `2^(2*width)`. Constructing the
current explicit jump kernel still takes `O(width * 2^width)` work and
`O(2^width log horizon)` entries. The experiment therefore removes an avoidable
exponential factor without claiming polynomial scaling in arbitrary width.

The 36-case phase grid used widths 4, 8, and 12 and horizons 10, 100, and 1,000.
All rows agreed with persistent Varisat and produced valid witnesses; measured
query speedups ranged from 19.82x to 7,746.55x. The independently seeded 36-case
holdout used widths 5, 9, and 13 and horizons 37, 333, and 2,000. Again every row
agreed and every witness validated, with speedups from 52.26x to 17,387.40x.
The largest instance contained 26,013 CNF variables.

This is a meaningful model-checking improvement for repeated deterministic
systems with locally defined outputs. The next boundary is avoiding the explicit
`2^width` state table, for example through a symbolic transition representation.

## Symbolic local-function replay

The next prototype removes the explicit `2^width` state table entirely. It keeps
the recovered local truth tables as a symbolic transition circuit and evaluates
only the requested trajectory. With bounded dependency size, representation is
linear in width and independent of the total state-space size.

The 24-case phase grid covered widths 16 and 32 and horizons 10, 100, and 500.
All answers agreed with persistent Varisat and all complete trajectories
validated, with query speedups from 12.87x to 100.37x. The unseen 36-case holdout
covered widths 24, 48, and 64 and horizons 37, 333, and 1,000. It achieved 36/36
agreement and witness validity, with speedups from 18.42x to 184.18x. The largest
formula contained 64,064 variables. At width 64, representation used only 704
entries for the three-input families or 1,280 for the four-input family; an
explicit state table would require `2^64` states.

This result applies to deterministic queries with a fully specified initial
state, optionally plus observations at later times. It reconstructs the complete
trajectory in `O(width * horizon)` time. It does not yet solve existential search
over a partially specified initial state, nor provide logarithmic-horizon jumps.
The next boundary is symbolic function composition or symbolic preimage search.

## Exact symbolic preimages

The next experiment crosses the partial-initial-state boundary. Every state bit
at every frame is represented as a BDD over only the initial-frame variables.
Query observations are intersected symbolically, one satisfying initial state is
extracted, and the local transition circuit replays it to reconstruct the full
trajectory. No initial-state enumeration is used.

All 48 phase cases (widths 4, 6, and 8; horizons 2, 4, 8, and 16) were admitted,
agreed with persistent Varisat, and produced valid witnesses. Query performance
ranged from 0.50x to 42.47x Varisat. The separately seeded 48-case holdout used
widths 5, 7, and 9 and horizons 3, 7, 15, and 31. It also achieved complete
agreement and witness validity, with speedups from 0.78x to 57.22x. The largest
holdout BDD contained 123,160 nodes.

The result is exact but conditional on BDD compactness. A fixed 200,000-node gate
rejects growth before answering; it never approximates. Several families reached
fixed points or cycles, causing BDD size to stabilize, while the four-input
cascade continued growing. Short-horizon cases can be slower than Varisat, but
the reusable symbolic preimage becomes advantageous on most longer horizons.
This is not polynomial scaling for arbitrary transitions: BDDs can still require
exponential space under an unfavorable function or variable order.

## Calibration-free BDD ordering

Four fixed ordering rules were compared on a 24-case phase cohort: natural,
reverse, even/odd, and a dependency-graph traversal computed without trial
solves. All remained exact and admitted every phase case. Dependency ordering
tied natural at 298,679 aggregate nodes because the rotationally symmetric ring
families give the graph heuristic no asymmetry to exploit. Reverse used 301,237
nodes and even/odd used 303,437. There was no phase improvement to claim.

The dependency rule was nevertheless preselected for a 36-case unseen holdout.
It admitted 35 cases with exact agreement and valid witnesses. The width-10,
horizon-29 cascade was rejected exactly when it crossed the 200,000-node gate at
frame 23. This confirms the gate and ordering implementation, but not a growth
breakthrough. A useful next ordering experiment requires asymmetric dependency
graphs or dynamic reordering with a correctness-preserving resource bound.

## Asymmetric dependency ordering

Three asymmetric transition families were added: hub-coupled, tree-like, and an
irregular modular dependency graph. Each remains a repeated deterministic local
composition, so the exact preimage and witness contract is unchanged.

On the 18-case phase cohort, all four orders admitted every case and all 72 rows
were exact. Dependency ordering used 23,288 aggregate BDD nodes versus 25,286 for
natural order, a 7.90% reduction. Reverse used 25,041 and even/odd used 28,985.
Dependency order also reduced the phase maximum from 4,389 to 3,729 nodes.

Dependency order was frozen before the unseen even-width holdout. All 27 selected
rows were admitted, agreed with Varisat, and returned valid witnesses. Against a
natural-order control on that same holdout, dependency order used 25,026 aggregate
nodes versus 25,597, a smaller 2.23% improvement. It was not better on every
formula, but the aggregate phase advantage survived without per-formula trials.
This supports dependency ordering as a cheap default for asymmetric graphs, not
as a universal solution to BDD growth.

## Exact frame-cycle checkpoints

The preimage compiler now hashes each complete vector of state-bit BDD roots.
Because the manager is canonical for a fixed variable order, an identical vector
is an exact repeated symbolic state function. On repetition, compilation stops
and later frames map into the stored cycle by modular arithmetic. No equivalence
guess or approximate fingerprint is used.

All 24 phase cases at widths 6 and 8 found cycles for horizons 100, 1,000, and
10,000. They compiled only 7--21 unique frames, agreed with Varisat, and produced
valid full trajectories. Query speedups ranged from 90.17x to 3,239.19x.

The odd-width holdout admitted 33 of 36 cases through horizon 7,777. Admitted
instances compiled at most 57 frames, all answers agreed, and all witnesses
validated; speedups ranged from 51.60x to 5,015.53x. The three width-9 cascade
instances exceeded 200,000 BDD nodes at frame 60 before a cycle was found and
were rejected exactly. Cycle checkpoints therefore remove horizon-dependent BDD
composition and frame storage after a repeat, but do not prevent pre-cycle BDD
explosion. CNF normalization and complete witness output also remain linear in
the supplied horizon.

## Calibration-free pre-cycle growth guard

A fixed guard now compares consecutive exact BDD node increments. Once growth is
accelerating, it projects four more increments; if that projection exceeds the
hard node budget, compilation rejects immediately. The guard changes only
admission timing, never SAT semantics or witness reconstruction.

On the 24-case cycle phase cohort, the guard admitted the same 24 cases as the
unguarded dependency order, with complete agreement and witness validity. On the
36-case holdout it admitted the same 33 cases and rejected the same three
width-9 cascades. The guard rejected those at frame 56 with 192,220 nodes rather
than reaching the 200,000-node limit at frame 60. Aggregate rejection time fell
from 400.57 ms to 372.07 ms, a 7.12% reduction, with no observed false rejection.

This is bounded resource protection, not a cure for pre-cycle explosion. A more
aggressive projection could reject a formula shortly before it stabilizes, so
the guard remains an explicit optional mode rather than the default exact gate.

## Exact BDD/CDCL representation switch

The `hybrid` mode runs dependency-ordered, growth-guarded BDD compilation first.
If and only if the fixed growth guard fires, it constructs a persistent CDCL
representation of the same temporal CNF and serves the query workload there.
Other recognition errors still reject. This is an exact backend switch after the
guard, not conversion of the partially built BDD into an AIG.

All 24 phase cases remained on the BDD backend, agreed with Varisat, and returned
valid witnesses, with speedups from 89.66x to 4,351.02x. On the 36-case unseen
holdout, 33 remained on BDDs and the three former width-9 cascade rejections used
CDCL fallback. The hybrid therefore admitted 36/36 cases with complete agreement
and witness validity. Overall holdout speedups ranged from 0.97x to 5,282.56x;
the three fallback rows were 0.97x--1.03x baseline, as expected for equivalent
persistent CDCL work after paying the failed BDD attempt.

This closes the operational completeness gap but does not make explosive cases
faster. The next representation experiment should reuse the already composed
prefix—via an exact circuit/AIG checkpoint—instead of restarting from the full
CNF when the guard fires.

## Exact BDD-prefix CDCL checkpoint

The follow-up composes an exact BDD prefix to a fixed frame, converts every
canonical BDD node to Tseitin CNF, links the stored symbolic frames to temporal
variables, and appends only the original suffix clauses. Arbitrary observations
before and after the checkpoint remain supported, and full models are recovered.

All phase and holdout answers agreed with full persistent CDCL and every witness
validated. Performance was negative. Fixed checkpoints 10, 20, and 40 on the
width-9 cascade used 41,299, 85,761, and 149,962 BDD nodes respectively. Frame 10
was preselected, but achieved only 0.078× and 0.379× speedups on horizons 137 and
1,333. On the unseen horizon-7,777 holdout it improved to 0.727× but did
not break even.

This proves exact prefix reuse is possible, but naïvely exposing four Tseitin
clauses per BDD node gives CDCL a much larger, less native representation than
the original transition CNF. The next checkpoint experiment should compact or
slice the BDD roots to only queried observations, or translate the prefix to a
shared AIG with structural hashing before CNF emission.

## Structurally hashed AIG checkpoint

The AIG follow-up converts each BDD decision to shared AND/inverter logic, applies
constant, identity, complement, and commutative reductions, and emits three CNF
clauses per surviving AND gate. It preserves arbitrary observations and full
witness recovery exactly.

It did not compact the cascade prefix. At width 9 and checkpoint 10, 41,299 BDD
nodes became 83,971 AND gates and 251,913 clauses. Speedups versus full CDCL were
0.051× and 0.221× at horizons 137 and 1,333. The preselected unseen horizon-7,777
holdout reached 0.619×, below the direct BDD checkpoint's 0.727×. All answers
agreed and all witnesses validated.

Generic AIG structural hashing therefore fails on this prefix because canonical
BDD sharing does not translate into repeated two-input conjunctions. The next
experiment should lazily encode only the checkpoint cone plus earlier frame roots
actually mentioned by queries, retaining exactness while avoiding unused history.

## Lazy observation-cone checkpoint

The lazy checkpoint emits only the checkpoint frame's reachable BDD cone at
recognition. An observation at an earlier frame recursively adds its missing cone
once and is passed to CDCL directly as an assumption on the corresponding BDD
root. Unobserved prefix variables never enter the CNF; after solving, every prefix
frame is evaluated from the recovered initial state to reconstruct a full witness.

This is exact and materially compact. Across the fixed width-9, checkpoint-10
cohorts, only 793–1,252 of 41,299 BDD nodes were needed: a 97.0–98.1% reduction.
All answers agreed with full persistent CDCL and every reconstructed witness
validated.

Runtime did not generalize reliably. Individual phase measurements crossed the
baseline in both directions; one horizon-1,333 run reached 1.189×, while a repeat
with direct root assumptions reached 0.672×. The preselected horizon-7,777 holdout
was 0.878× before and 0.876× after removing permanent observation links; a larger
200-query batch stabilized at 0.874×. Lazy cones are therefore a representation
breakthrough, not yet a solver-speed breakthrough. The remaining target is a
native circuit propagator or a compact encoding whose propagation quality matches
the original transition clauses.

## Native BDD-theory bridge

The native bridge avoids checkpoint CNF entirely. For each query it builds the
prefix observation constraint in the BDD, sends forced checkpoint literals to a
persistent suffix CDCL solver, checks the proposed checkpoint state against the
BDD, and learns an activation-gated blocking clause for each incompatible state.
The activation makes query-specific learning retractable without rebuilding the
solver. Full witnesses are reconstructed from a satisfying initial BDD path.

The bridge is exact, but reconciliation dominates. Without bounded correlation
clauses it learned 184 and 120 checkpoint conflicts across the horizon-137 and
horizon-1,333 phase rows, reaching only 0.703× and 0.615× speedups. Adding all
query-specific pairwise consequences produced only 16 useful clauses on the
short row and none on the longer rows; conflict counts were unchanged. Pairwise
phase speedups were 0.639× and 0.608×, and the unseen horizon-7,777 holdout was
0.700× with 101 conflicts. Every answer agreed and every witness validated.

The checkpoint relation is therefore higher-order: unary and binary propagation
cannot convey enough of the native BDD to CDCL. A credible next step is conflict
generalization—derive a smaller BDD explanation for each rejected full state—so
one learned clause excludes a whole incompatible subcube rather than one state.

## BDD conflict generalization

For every rejected checkpoint state, the generalizer starts with all checkpoint
literals and greedily removes a literal whenever the remaining conjunction is
still inconsistent with the query's prefix BDD. The resulting activation-gated
clause is exact and blocks an entire incompatible subcube.

Explanation quality improved. On the width-9 phase, average learned width fell to
5.86 and 6.23 literals (maximum 7), while conflicts fell from 184 to 153 and from
120 to 104. The horizon-1,333 speedup improved from the pairwise bridge's 0.608×
to 0.895×. The short phase reached 0.668×.

The unseen horizon-7,777 holdout learned width-6.18 clauses and reduced conflicts
from 101 to 89, but speed fell from 0.700× to 0.546×. Repeated BDD conjunctions
during greedy deletion cost more than the stronger clauses save. The explanation
idea works; its construction does not. The next experiment should extract a
conflict reason during a single BDD traversal or cache subset cofactors, avoiding
one fresh conjunction chain per deletion candidate.

## Cached BDD conflict explanations

The cached extractor precomputes suffix conjunctions of the proposed checkpoint
literals, carries a retained prefix constraint, and tests every literal exactly
once. It remains exact and produces the same generalized subcubes as the repeated
deletion implementation on all measured rows.

Clause quality and reconciliation counts were unchanged: average learned widths
were 5.86, 6.23, and 6.18, with 153, 104, and 89 conflicts. Runtime did not improve
robustly. Phase speedups were 0.603× and 0.757× versus the earlier 0.668× and
0.895×. The unseen horizon-7,777 holdout improved from 0.546× to 0.596×, still far
below full CDCL.

BDD apply caching had already absorbed much of the apparent quadratic rebuilding
cost. The dominant cost is now repeated CDCL-to-BDD reconciliation. The next
experiment should move from post-model conflict explanations to pre-solve
higher-order propagation, or compile a bounded set of generalized global clauses
once and reuse them across every query.

## Reusable global checkpoint clauses

The global compiler enumerates the bounded checkpoint state space once, asks the
BDD whether each state is reachable from any initial state, skips states already
covered by a learned reason, and generalizes every remaining unreachable state
into an exact permanent clause. Query-specific prefix observations still use the
native BDD, but CDCL receives the reusable checkpoint-image relation before it
proposes a model.

This is the first robust runtime crossover. The width-9, checkpoint-10 cascade
compiled 134 global clauses averaging 6.28 literals. The BDD grew from 41,299 to
51,664 nodes during offline compilation. Reconciliation fell to two conflicts on
horizon 137 and zero on horizons 1,333 and 7,777. Speedups versus full persistent
CDCL were 2.546×, 1.190×, and 2.601×. Recognition-inclusive break-even occurred
after approximately 6.8, 4.8, and 0.5 queries. All answers agreed and all full
witnesses validated on the preselected unseen holdout.

This does not address arbitrary SAT or P versus NP. It demonstrates a practical
solver architecture for repeated queries over recognized deterministic temporal
systems with a small checkpoint width: bounded model checking, protocol and
hardware trace verification, planning, and fault diagnosis. The next requirement
is external validation on independently sourced transition systems and wider
checkpoints, with an admission gate for the exponential checkpoint enumeration.

## Asymmetric cross-family validation

The first structural generalization matrix holds checkpoint 10 and 50 queries
fixed while testing `hub3`, `tree3`, and `irregular3` at widths 7, 9, and 11 and
horizons 137 and 1,333. All 18 rows agreed with full CDCL and validated every
returned witness. Global clauses eliminated reconciliation conflicts on 17 rows;
the remaining irregular width-7 short row needed two query-local explanations.

Runtime generalization is mixed. Six of 18 rows beat full CDCL, and five amortized
recognition within the 50-query batch. Hub width 7 reached 1.305× and 1.716×;
long width-11 rows reached 1.342× for hub, 1.092× for tree, and 1.117× for
irregular. Most other rows ranged from 0.503× to 0.997×. The strong cascade result
therefore identifies a real regime, not a broadly dominant solver.

The practical conclusion is conditional: reusable checkpoint-image clauses can
accelerate repeated temporal verification when the eliminated prefix work exceeds
the BDD/theory overhead. A calibration-free admission gate must reject the other
regimes before deployment. Candidate static features are checkpoint width, suffix
horizon, BDD nodes per state bit, global-clause count and width, and original
prefix-clause volume; evaluation must preselect the rule and test unseen families.

## Calibration-free CQ-SAT/GCC portfolio

The released gate is deliberately conservative and explainable. It selects the
global-checkpoint backend only for dense transitions up to width nine with at
least eight declared queries, or hub-like dependency graphs up to width seven
with at least 128 declared queries. All other recognized models use the identical
persistent-CDCL implementation.
The gate reads no solve times, candidate answers, or formula-specific calibration.

The broad `long-wide` candidate was rejected before release. On identical mixed
width-12/horizon-2,049 formulas, independent 200-query seeds produced approximately
1.03× and 0.85× native-backend speedups. Formula structure alone could not safely
distinguish them. Removing that rule turns the three unseen majority, multiplexer,
and mixed-dynamics cohorts into exact CDCL fallbacks rather than selected losses.

Dense-transition stability runs reached 1.91× and 1.99× across independent
200-query seeds. Narrow-hub runs reached 1.19× and 1.14×, with conservative
break-even near 96–108 queries, motivating the 128-query threshold. In the
executable watchdog example, the portfolio achieved recognition-inclusive
speedups of 2.03×, 1.68×, and 3.06×. The sensor-voting example selected CDCL and
matched its query path at 1.0×. All portfolio and oracle rows agreed and validated
complete witnesses.

This is useful today as a research-grade repeated temporal-verification and
bounded AIGER safety runner, not as a general DIMACS auto-solver.

## External ASCII AIGER validation

The first standard interchange path parses and validates closed deterministic
ASCII AIGER (`aag`) models with at most nine latches. It evaluates the AIG exactly,
constructs the repeated latch transition CNF, preserves declared initial values,
and converts bad-state outputs into bounded reachability queries. Models with
primary inputs are rejected rather than unsoundly treated as deterministic.

An independently authored four-bit counter-overflow model from Tobias Nießen's
MIT-licensed AIGER safety suite exposed a gate counterexample. Transition density
alone selected CQ-SAT/GCC, whose query path was approximately 0.22× the CDCL
baseline on the first 50 property queries. Full-state property enumeration has a
different cost profile from sparse trace observations. The v0.3.0 gate
therefore also reads average assumption count—a static property of the declared
batch—and rejects batches averaging more than one state-width of assumptions.
With that correction the external model selects exact CDCL, agrees on all 50
sampled queries, validates all witnesses, and finds three reachable bad-state
queries. The exhaustive 137-query safety run reports `UNSAFE` at frame 15, finds
eight bad frames through frame 137, and emits the complete counterexample trace.

This is the first independently sourced model exercised through the portfolio.
It validates exact bounded-safety reporting and safe selection, not specialized
acceleration.

## Input-driven AIGER bounded model checking

Primary-input and wider AIGER models now follow an exact scalable fallback. Each
time frame receives explicit input, latch, and AND-gate variables. Three-clause
Tseitin equivalences encode every AND gate, two-clause equivalences connect latch
updates, and declared initial values become unit clauses. One aggregate clause
asks whether any bad output is true in any frame. A SAFE result is therefore one
UNSAT proof across every input sequence; an UNSAFE result is accompanied by a
complete input/latch trace. Binary search over the bound produces the shortest
bad horizon without changing satisfiability semantics.

Two independently sourced models validate both outcomes:

- Peterson's two-thread, one-core mutual-exclusion model has 2 inputs, 9 latches,
  and 109 AND gates. Its forbidden simultaneous-critical-section output is SAFE
  through frame 100 for every scheduler and signal sequence. The encoding has
  12,120 variables and 34,836 clauses.
- The eight-bit SPI receiver has 3 inputs, 18 latches, and 71 AND gates. It is
  UNSAFE within the requested 50-frame bound; minimization finds frame 16 and
  emits the complete `SCLK`, `MOSI`, `NOT_CS`, and latch trace.

Both workloads are rejected from CQ-SAT/GCC statically with
`aiger-primary-inputs`; no candidate solve or timing calibration occurs. This is
a real use of the portfolio as a bounded hardware/protocol safety verifier, while
specialized acceleration remains limited to the deterministic repeated regime.

## RTL-to-safety product boundary

The v0.6 product path removes the hand-authored AIGER step from the infusion-pump
workflow. It stages one SystemVerilog source file, invokes Yosys through a fixed
script without interpolating source-controlled command text, and lowers an
explicit `bad` output to the original five-field ASCII AIGER subset. CQ-SAT/GCC
then verifies that generated model through its existing exact portfolio.

Yosys symbols now survive into the report. The regressed controller's shortest
counterexample is named directly as `requested_motor_active`, `motor_request`,
and `door_open`, with the failure at frame 1. The protected controller is SAFE
through the chosen bound. SymbiYosys with Z3 independently returns PASS and FAIL
for those same sources and locates the regression at step 1.

This is a product-integration result, not a specialized-backend speed result:
primary inputs route both models to exact CDCL. The accepted boundary is one
source, one simple top identifier, and an explicit top-level `bad` output.
Source size is capped at 10 MiB and Yosys synthesis at 120 seconds before the
existing generated-model and bounded-unrolling limits are applied.
General AIGER 1.9 property/constraint sections and full source-level assertion
mapping remain unsupported and are rejected rather than approximated.

## Multi-module repeated-property scaling

The next product-shaped model contains five cooperating modules: command
sequencing, dose accounting, watchdog timing, sensor voting, and the top-level
pump system. Yosys lowers it to 8 inputs, 14 latches, 4 safety outputs, and 238
AND gates. This exposed and fixed a real v0.6 integration gap: hierarchical cells
must be flattened before AIGER export. Synthesis don't-care bits are now lowered
explicitly to zero; unconstrained top-level signals remain primary inputs.

The corrected benchmark asks one reachability question per property, aggregating
all frames through each horizon. Every timing repetition starts a new batch, so
learned clauses do not leak between repetitions. Bounded reuse shares a solver
for two properties; cold BMC rebuilds it for each property. All 160 curated
queries agree and all four properties are SAFE.

On the recorded release-mode run, full end-to-end reuse speedup is 1.27x, 1.20x,
and 1.06x at horizons 8, 16, and 32. At horizon 64 it reverses to 0.64x: learned
solver state costs more than rebuilding. Three additional runs reproduced the
same boundary. The static portfolio therefore enables reuse only for at least
two properties and at most 25,000 base clauses. It selects cold BMC at horizon
64 and on the independent single-property Peterson and SPI fixtures, yielding a
measured no-regression selection on this corpus. This is a bounded engineering
result, not evidence that reuse universally dominates BMC.

## Retraction and correction

Early independent-update tests suggested fast suffix-only clause deletion.
Cumulative workloads exposed a counterexample: distinct source clauses may
collapse to the same canonical residual. Without provenance multiplicity,
deleting one source clause may incorrectly remove a constraint still contributed
by another clause. The implementation now rebuilds from the root on deletion.
Only insertion repair is claimed as local and exact.

## Appropriate interpretation

This work identifies a useful specialized knowledge-compilation regime:

1. the formula passes the structural gate;
2. its structure and query regime remain stable;
3. many partial-assignment queries require complete witnesses.

It is not a general SAT breakthrough and makes no P-versus-NP claim.
