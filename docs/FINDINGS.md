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
