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
