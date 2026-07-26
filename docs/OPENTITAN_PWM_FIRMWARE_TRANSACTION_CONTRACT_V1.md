# OpenTitan PWM firmware transaction-contract experiment v1

Status: predeclared experiment. No result or novelty claim exists.

## Product question

Can GCC carry one independently verified firmware programming contract into an
authentic multi-file RTL revision-impact analysis, reuse that contract across
every revision combination and property, and still expose OpenTitan's
inter-channel-crosstalk defect?

This is a cross-layer experiment. It does not infer assumptions from solver
answers and it does not replace RTL checking with a firmware model.

## Authentic contract source

OpenTitan's
[PWM programmer's guide](https://opentitan.org/book/hw/top_earlgrey/ip_autogen/pwm/doc/programmers_guide.html)
prescribes the fixed-channel sequence:

1. disable blinking;
2. set duty cycle;
3. optionally set phase delay;
4. optionally set polarity; and
5. enable the channel.

The guide also states that changes take effect immediately. Configuring another
channel while the first channel is running is therefore valid firmware
behaviour, not an environment violation.

Version 1 freezes one two-channel transaction:

```text
configure channel 0
enable channel 0
configure channel 1 while channel 0 remains enabled
observe channel 0
```

This is the schedule relevant to the retained OpenTitan revision
`86db2898288664d8d5e8fc635b48951ef63e3439`, whose purpose is to eliminate
inter-channel crosstalk.

## Frozen monitor

The monitor is a deterministic five-state automaton:

```text
Reset
  -> Channel0Configured
  -> Channel0Enabled
  -> Channel1Configured
  -> ObservationReady
```

Each transition consumes an explicitly named firmware event. Any event that
skips a state, repeats an already consumed configuration step, disables
channel 0 before observation, or writes channel 0 while channel 1 is being
configured enters a permanent rejected state.

The accepted trace must reach `ObservationReady`. A proof that the contract is
never violated without a separate reachability witness is vacuous and is
reported as `NONE`.

## Proof-carrying composition

The candidate envelope binds:

- the exact contract text and its SHA-256 digest;
- the canonical monitor transition table;
- one reachability witness for `ObservationReady`;
- one SAFE proof that the retained firmware event trace never enters the
  rejected state;
- the existing source-bound two-atom revision-impact bundle;
- the exact mapping from contract events to the revision fixture's stimulus;
- every revision mask, query, result and minimal semantic-change set; and
- format version, static resource limits and section digests.

The checker independently reconstructs the monitor, replays both contract
claims, verifies the revision-impact bundle, and confirms that every query
uses the same accepted schedule. A valid contract cannot turn an RTL
counterexample into SAFE. Unsupported or unbound inputs use unchanged exact
fallback or return no logical answer.

## Predeclared cohort

The primary cohort retains:

- five existing query classes;
- four old/new atom combinations;
- one valid two-channel firmware schedule;
- four invalid schedules, each violating a different transition; and
- both the parent and child RTL revisions.

The existing 20-result matrix must remain unchanged under the valid contract.
The old core must still fail the core-only and joint crosstalk properties. If
the contract masks those failures, the candidate fails.

Invalid schedules are contract refusals, not SAFE RTL results.

## Hostile controls

The implementation must reject:

- contract text or digest substitution;
- event reordering, omission, duplication and trailing events;
- a missing observation-ready reachability witness;
- a SAFE monitor certificate replayed against another schedule;
- stimulus mapping drift;
- revision source, atom order, interface, query or result drift;
- a contract envelope containing a valid but unrelated impact bundle;
- section truncation, extension and single-byte mutation; and
- work exceeding static event, evidence, artifact or verification limits.

No partial envelope may be published after any refusal.

## Baselines and gates

Compare the complete candidate with:

1. the existing GCC revision-impact path without a firmware contract;
2. pinned Yosys, rIC3 and Certifaiger over the identical contracted source
   combinations and properties; and
3. direct monitor replay plus the maintained RTL result table.

All parsing, monitor construction, reachability, proof, composition,
verification and fallback work remains in the comparison.

The experiment advances only if:

1. the valid schedule preserves all 20 existing answers and three minimal
   semantic-change sets;
2. the old-core crosstalk failure remains observable;
3. every invalid schedule refuses without an RTL answer;
4. every hostile control fails closed;
5. two clean productions are byte-identical; and
6. independent checking is cheaper than rebuilding all contract and RTL
   evidence separately.

There is no predeclared speed or byte threshold. Negative results remain in the
repository.

## Claim boundary

Protocol automata, transaction monitors, assume-guarantee verification,
contract-based design, proof-carrying code and compositional model checking
have substantial prior art. Passing this experiment establishes a bounded
cross-layer product capability only. Novelty remains prohibited until a
distinct proof-reuse invariant survives an equivalent-scope closest-system
comparison and independent expert review.
