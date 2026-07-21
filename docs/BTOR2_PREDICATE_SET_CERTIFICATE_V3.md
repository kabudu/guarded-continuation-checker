# BTOR2 predicate-set certificate v3

Status: experimental additive Rust API and self-service CLI. Not a production
release.

## Purpose

Predicate-set v3 composes exact bounded predicates across two or more reset-add
recurrences when at least one recurrence is enabled by a separately proved
constant state invariant. It preserves ordered SAFE and UNSAFE results and the
earliest bad frame for every UNSAFE member in one source-bound artifact.

The initial admitted language is deliberately narrow:

- one semantic one-bit reset input;
- 3 to 16 word states and no BTOR2 constraints;
- constant initial and reset values;
- direct reset-add recurrences or hold/add recurrences whose guard is proved
  constantly true by one admitted invariant;
- equality or unsigned-greater-than-or-equal bad predicates;
- at least two recurrence states and one used guard invariant; and
- no admitted arithmetic wrap through the requested horizon.

Unsupported queries preserve the complete ordered query through ordinary exact
v3 fallback. If any fallback member exceeds its governed bound, production
fails without returning partial evidence.

## Certificate and checking contract

The `invariant_chained_regions` artifact records the exact source digest, query
horizon, semantic reset input, logical state count, invariant claims,
recurrence claims, and ordered predicate members. UNSAFE members use the
source-reconstructed `advance_prefix` witness; SAFE members carry no witness.

Verification reparses the exact source and reconstructs the invariant,
recurrences, predicates, results, and earliest frames. Encoded claims are not
trusted as axioms. Query omission, duplication, reordering, horizon changes,
source substitution, forced ordinary downgrade, malformed text, trailing
fields, and incomplete artifacts fail closed.

The static route order is:

1. `invariant_chained_regions` for the complete admitted multi-recurrence set;
2. retained v2 `shared_exact_region` for a complete one-recurrence set; or
3. `ordinary_exact` v3 for the complete unsupported set.

Selection does not use elapsed time, training, or per-formula calibration.

## Versions and bounds

- current certificate version: `3`
- current portfolio version: `3`
- supported certificate and portfolio versions: `1,2,3`
- maximum members: 64
- maximum states considered by invariant chaining: 16
- maximum horizon: 1,000,000,000
- maximum BTOR2 source: 8 MiB
- maximum complete artifact: 64 MiB
- canonical UTF-8, LF-only text without NUL or trailing fields
- no-clobber, mode-0600 CLI publication on Unix

The public producer may still emit a v2 shared artifact when that older route
is the canonical result. `certificate_version` and `portfolio_version` report
the actual artifact, while the discovery command reports v3 as current.
Retained v1 and v2 artifacts keep their original routing and checking semantics.

## OpenTitan acceptance

The pinned OpenTitan AON dual-timer model contains a 12-bit prescaler invariant,
a direct 32-bit watchdog recurrence, an invariant-guarded 64-bit wake-up
recurrence, and three properties. Retained v3 artifacts preserve:

| Horizon | Ordered answers | Bytes |
| ---: | --- | ---: |
| 4 | SAFE, SAFE, SAFE | 445 |
| 5 | SAFE, UNSAFE at 5, SAFE | 454 |
| 7 | SAFE, UNSAFE at 5, UNSAFE at 7 | 463 |
| 9 | UNSAFE at 9, UNSAFE at 5, UNSAFE at 7 | 472 |
| 1,000,000,000 | UNSAFE at 9, UNSAFE at 5, UNSAFE at 7 | 515 |

Run `scripts/run-opentitan-aon-dual-timer-acceptance.sh` to regenerate the
pinned model, reproduce every artifact, verify retained v1/v2 compatibility,
and execute the hostile controls. The separate per-property GCC baseline cannot
answer the input-dependent wake predicate and is reported as unavailable, not
as a compression win.

Pinned official BTOR2Tools locally parses the model and replays each isolated
bad-boundary witness. Maintained SMT agreement, Linux resource enforcement,
three-platform downstream API checks, and hosted reproduction remain open. A
same-width cross-coupled near-neighbour is retained as a rejected unit control.
Invariant reasoning, recurrence
acceleration, and multi-property checking have substantial prior art; v3 is not
yet a novelty claim.
