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
