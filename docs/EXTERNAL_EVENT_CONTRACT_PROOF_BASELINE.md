# External event-contract proof baseline

This experiment exports every certificate v3 relation and terminal
completeness claim to canonical DIMACS, replaces the native Varisat proof
producer with CaDiCaL 3.0.0, and checks text DRAT with DRAT-trim v05.22.2023.
It crosses both the producer and proof-checker implementation boundaries while
preserving the exact event-contract obligations.

## Export contract

`export-aiger-event-contract-v3-obligations` accepts the source AIG, original
named contract, v3 certificate, and a new output directory. It reparses all
three inputs and binds the bundle to their SHA-256 digests. It rejects source,
contract, dimension, phase, predicate, or overwrite mismatches.

The no-clobber bundle contains one DIMACS file per phase and source state, one
terminal DIMACS file, a selector aggregate, and a canonical manifest. The
aggregate uses disjoint variable blocks. It is UNSAT exactly when every
individual obligation is UNSAT. Individual files are retained and checked too.

The exporter is deterministic across both answer classes. Tests independently
solve every emitted CNF, verify the aggregate equivalence, reject a substituted
contract, and confirm that a second export cannot replace existing evidence.

## Maintained baseline

The baseline pins:

- CaDiCaL commit `7b99c07f0bcab5824a5a3ce62c7066554017f641`;
- DRAT-trim commit `2e5e29cb0019d5cfd547d4208dca1b3ec290349f`;
- a 300-second deadline per solver or checker process;
- a 2 GiB address-space ceiling; and
- a 512 MiB proof-file ceiling.

GNU `timeout` is used when available. macOS can use `gtimeout`; otherwise the
harness uses a Perl alarm with the same deadline. Solver status must be 20,
checker status must be zero, and DRAT-trim must emit `s VERIFIED`.

## Result

All 68 individual obligations and four selector aggregates verified across the
three product-shaped avoidable cases and one satisfiable unavoidable case.
Aggregate production plus checking took 39.460 to 45.619 ms on the measured
Apple Silicon host. This is much slower than the native v3 checker, but it adds
format and implementation diversity rather than query speed.

Raw rows and exact reproduction instructions are in
[`results/external-event-contract-proof-v3`](../results/external-event-contract-proof-v3/README.md).

## Claim boundary

This closes the declared maintained external proof-checker gate for v3
completeness obligations. It does not externally validate witness replay,
relation powering, final composition, or the result bit, which remain duties of
the independent v3 verifier. It is not a formally verified checker, an external
expert review, a complete Certifaiger comparison, or proof of scholarly
novelty.
