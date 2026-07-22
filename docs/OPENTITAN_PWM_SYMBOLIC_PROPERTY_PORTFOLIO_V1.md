# OpenTitan PWM symbolic property portfolio v1

## Question

Can a source-replayed structural admission capability safely reduce actual
bounded property evidence across PWM channels with live firmware inputs, while
preserving exact singleton checking and reconstructable UNSAFE inputs?

## Contract

The experiment uses the authenticated symbolic-class models and their canonical
structural admission artifacts. Each workload asks two Boolean observation
properties for every channel:

- `OutputHigh`: a high PWM observation is the bad condition.
- `OutputLow`: a low PWM observation is the bad condition.

These opposing properties force both answer classes at the retained horizon.
They are mechanism checks, not claims about a real PWM safety specification.

Queries are strictly identifier ordered and bind channel, property kind, and
horizon. One exact bounded-search certificate is produced for each verified
class and query shape. A non-singleton member uses its canonical representative;
a singleton remains a direct exact member. Verification replays the structural
artifact from separately supplied source, reconstructs each representative
property model, decodes and verifies every exact certificate, and refuses
query omission or reordering. For every derived UNSAFE answer, it replays the
certificate's input valuations against the target channel property model and
requires the target violation to occur.

Invalid structural admission is an error and never triggers fallback. Forced
backend changes, evidence drift, query drift, and source drift also fail closed.

## Retained horizon-1 result

| Channels | Logical queries | Proof members | Reused queries | Direct evidence | Retained evidence | Reduction |
|---:|---:|---:|---:|---:|---:|---:|
| 2 | 4 | 4 | 0 | 1,840 B | 2,072 B | -12.61% |
| 4 | 8 | 6 | 2 | 4,056 B | 3,390 B | 16.42% |
| 6 | 12 | 6 | 6 | 6,636 B | 3,778 B | 43.07% |

All answers and earliest bad frames agree with twelve separately produced exact
certificates on the six-channel workload. The six `OutputHigh` queries are SAFE
through frame 1. The six `OutputLow` queries are UNSAFE at frame 0, and all six
target assignments replay successfully. The retained byte count includes the
structural admission artifact and member certificates, but not an outer
portfolio codec because that format does not exist yet.

The two-channel row is the required negative control: no class is reusable, so
the admission artifact adds 12.61% overhead. This rules out universal use based
only on the existence of a structural artifact.

## Resource refusal

The exact explicit-state backend proves the horizon-2 `OutputHigh` query for the
two- and four-channel models, but refuses the six-channel model under its frozen
20-million node-step limit. The portfolio propagates that error. It does not
raise the guard, return a partial batch, infer a SAFE answer from the horizon-1
result, or silently replace invalid specialised evidence.

This refusal is the next product blocker. A scalable exact word-level fallback,
with proof evidence and equivalent-scope maintained-tool comparison, is required
before the mechanism can support realistic PWM horizons.

## Reproduction

```console
scripts/run-btor2-symbolic-property-portfolio-probe-v1.sh /tmp/result.csv
scripts/check-btor2-symbolic-property-portfolio-probe-v1.sh /tmp/result.csv
cargo test --locked --test opentitan_pwm_symbolic_class_api
```

## Novelty boundary

The result combines established symmetry reduction with ordinary bounded-search
certificates. The evidence reduction follows directly from storing one member
per admitted class and does not establish scholarly novelty. Guarded equivalence
predicates, orbit representatives, finite-state certificates, and shared
multi-property verification are all close prior art. Candidate novelty remains
unproven and would require a semantic capability beyond that straightforward
combination, a maintained baseline, and independent expert review.
