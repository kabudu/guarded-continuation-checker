# BTOR2 component reuse v1 result

## Outcome

The local candidate passes the predeclared artifact and checking gates for fully
admitted batches of at least 2 members. It fails the universal-use hypothesis
when 25 percent of members require exact fallback.

At 64 admitted members:

- artifact size is 20,982 bytes instead of 31,793 bytes, a 34.0 percent reduction;
- median checking is 544,542 ns instead of 619,792 ns, a 12.1 percent reduction;
- production is 10.2 percent slower; and
- two verifications amortise production plus checking in the retained run.

Five repeated admitted-cohort runs preserved a checking advantage in every row.
At 64 members, the checking ratio ranged from 0.866 to 0.885. The mixed control
produced larger artifacts and no stable checking advantage.

## Interpretation

This establishes a bounded local performance result, not novelty or production
readiness. The result uses two repository-authored plant variants under one
controller. Required next evidence includes a public unmodified embedded design
family, maintained external-tool agreement, cross-platform replication, and a
static portfolio artifact that retains ordinary bundling for mixed batches.

Raw data is in [`btor2-component-reuse-v1.csv`](btor2-component-reuse-v1.csv).
