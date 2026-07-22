# OpenTitan PWM symbolic property portfolio v2

## Question

Can GCC choose between compact explicit-state evidence and proof-carrying
bitblast evidence without trial-solving a formula, then reuse the selected
exact proof across a source-verified firmware channel class?

## Static route

The v2 portfolio reconstructs each representative property model before any
solver runs. It projects explicit-state work from the model's input bits, node
count, state count, horizon, and frozen layer limits. Work within the existing
20-million node-step policy stays on explicit-state search. Work outside that
policy selects bounded CNF bitblasting only when its 64-bit input and 64-frame
limits admit the query. If neither exact backend admits the projected work,
production stops with an error.

The route does not measure either backend, learn from prior formulas, retry a
failed solve, or turn invalid specialised evidence into an answer. Verification
recomputes the same route from separately supplied BTOR2 and rejects a forced
solver label.

Explicit SAFE and UNSAFE members retain the bounded-search certificate. A
bitblast SAFE member carries a Varisat proof checked by `varisat-checker`; an
UNSAFE member carries a complete packed frame trace replayed by the word-level
BTOR2 interpreter. The portfolio additionally replays every derived UNSAFE
trace against each target channel rather than trusting representative class
membership alone. Target replay rechecks every environment constraint and
preserves the full governed 64-bit packed-input domain.

## Retained horizon-2 result

| Channels | Horizon | Logical queries | Proof members | Reused queries | Route | Direct evidence | Retained evidence | Reduction |
|---:|---:|---:|---:|---:|---|---:|---:|---:|
| 6 | 2 | 12 | 6 | 6 | 6 bitblast, 0 explicit | 1,500 B | 1,210 B | 19.33% |

All six `OutputHigh` properties are UNSAFE first at frame 2. All six
`OutputLow` properties are UNSAFE first at frame 0. Every target replay agrees.
The retained count includes the 460-byte structural admission and 750 bytes of
member evidence. The direct count contains twelve independently produced
125-byte bitblast witnesses. Neither count includes the subsequent
[outer property-portfolio codec](OPENTITAN_PWM_SYMBOLIC_PROPERTY_CODEC_V1.md).
Its complete 1,568-byte artifact is 4.53% larger than the twelve raw direct
witnesses after query metadata and the envelope checksum are included.

The earlier horizon-1 negative control remains binding. Bitblast SAFE proofs
are much larger than explicit-state evidence, and the two-channel structural
portfolio grows when there is no reusable class. V2 therefore establishes a
deterministic mixed-backend mechanism, not universal evidence reduction.

## Failure and resource behaviour

- Invalid structural admission, query drift, forced backend or solver drift,
  source drift, and changed member evidence fail closed.
- A projected fallback beyond the bitblast horizon is refused before solving.
- Member evidence is bounded individually by its backend and collectively by
  the portfolio's 64 MiB policy.
- No partial result is returned when any required member cannot be produced or
  verified.

## Reproduction

```console
scripts/run-btor2-symbolic-property-portfolio-v2-probe.sh /tmp/result.csv
scripts/check-btor2-symbolic-property-portfolio-v2-probe.sh /tmp/result.csv
cargo test --release --locked --test opentitan_pwm_symbolic_class_api
```

## Remaining gates

- The canonical, preflighted outer portfolio codec and frozen v1 compatibility
  fingerprint are now implemented.
- Govern aggregate production work before launching a large member batch, not
  only its retained evidence bytes.
- Measure generation, checking, peak memory, and package size on realistic
  safety properties and a larger operator corpus.
- Cross-check equivalent queries against maintained model checkers.
- Reproduce deterministic artifact identity on hosted Linux, macOS, and
  Windows.
- Obtain independent implementation and suitability review.

Static resource routing, bitblasting, proof checking, and symmetry-class reuse
are established techniques. Their integration here is product engineering and
does not establish a novel verification invariant.
