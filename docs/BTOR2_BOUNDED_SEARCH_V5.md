# BTOR2 bounded search certificate v5

Status: experimental implementation with local and hosted public-design
validation. No production result is claimed.

## Capability gap

Bounded search v4 preserves multiple Boolean controls and exact environment
constraints, but refuses every semantic input wider than one bit. Real embedded
interfaces commonly expose small register fields, sensor samples, counters,
opcodes, and bus values as words. Splitting a word into unrelated Boolean ports
would discard its source type and make canonical valuation reconstruction
dependent on synthesis details.

## Candidate semantics

Certificate v5 will admit one through eight semantic bit-vector inputs with at
most eight total input bits. It will bind, in canonical BTOR2 node order:

- every input node identifier;
- every declared input width;
- each field's implicit packed offset; and
- every ordered constraint node, including an explicit zero-constraint case.

Packed valuation bits are assigned by input-node order, then least-significant
bit first within each input. The verifier will reject width drift, offset
ambiguity, high bits, and any value outside the declared BTOR2 word width. It
will independently reconstruct every complete input word before checking
constraints, bad properties, and successors.

V5 will preserve the v4 all-frame assumption semantics, distinct terminal
valuation, empty dead-end suffixes, and exact SAFE layer closure. Static work
accounting will use `2^total_input_bits` before evaluating whether constraints
reduce the admissible set.

## Compatibility and governance

V1 through v4 production and encoding remain unchanged. V5 is selected only
when at least one semantic input is wider than one bit. The existing horizon,
layer-state, total-state, node-step, artifact-byte, and input-count limits stay
in force. The new total-bit limit is eight. Zero-width inputs, inputs wider than
eight bits, a total width above eight, overflow, and any resource violation
return no logical answer or partial artifact.

An additive capability command will disclose the exact width, packing,
constraint, and refusal contract. The CLI and public Rust API must preserve the
actual certificate version and complete source-bound input metadata.

## Predeclared gates

The cycle passes only if:

1. retained v1, Caliptra v2, PLIC v3, and constrained PLIC v4 results remain
   byte-identical;
2. v5 independently produces and verifies SAFE and UNSAFE word-input cases,
   both with and without constraints;
3. a separately implemented exhaustive oracle covers one-word, mixed Boolean
   and word, multi-word, input-dependent property, state-dependent constraint,
   and assumption-dead-end cases across every total width from two through
   eight bits;
4. input-node, width, total-width, valuation, constraint, terminal, layer,
   downgrade, truncation, and no-clobber attacks fail closed;
5. the static node-step gate refuses an otherwise valid word-input model before
   producing an answer or partial artifact;
6. a pinned public Caliptra watchdog workflow exposes a live two-bit timeout
   field, emits canonical word input semantics, and agrees with maintained
   Yosys plus Z3 on accepted horizons; and
7. the full hosted Linux suite and downstream API matrix pass.

## Claim boundary

Explicit enumeration of bounded bit-vector inputs is established model-checking
practice. V5 can close an important embedded-workflow integrity gap, but it is
not an algorithmic novelty claim. It provides the exact fallback needed to
measure later proof-carrying word-level composition honestly.

## Retained local result

The local core passes its width-binding, canonical packing, both-answer,
constraint, assumption-dead-end, resource, no-clobber, and downstream API
gates. A separately implemented exhaustive trace traversal agrees across one
word, mixed Boolean and word, two-word, state-dependent, input-dependent, and
constrained models for every total width from two through eight bits. Retained
v1 through v4 evidence remains byte-identical.

The pinned, unmodified Caliptra watchdog now receives a live two-bit timeout
period through GCC-owned formal instrumentation. The resulting canonical model
has digest
`b6e0b1db627d4daf3d03f617fd08f807b3b49b4f62b599843d65369047cc34ad`,
two semantic inputs with widths two and one, one state, one exact nonzero
constraint, and one bad property. V5 verifies SAFE at horizon zero and UNSAFE
at earliest frame one for horizons one and four. Two model builds and two
certificate builds are byte-identical. Seven width, binding, valuation,
constraint, downgrade, and truncation attacks fail closed, and output
publication does not clobber existing evidence.

Maintained Yosys plus Z3 independently proves the horizon-zero assertion and
finds the same frame-one violation. The retained rows are in
[`caliptra-wdt-word-input-acceptance-v1.csv`](../results/caliptra-wdt-word-input-acceptance-v1.csv).
[Hosted amd64 run
29874337371](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29874337371)
reproduces the pinned Caliptra model and all three retained result rows, checks
the SAFE and UNSAFE bounds with maintained Yosys plus Z3, preserves v1 through
v4 evidence, and passes the complete Linux suite, reproducible package,
dependency audit, Bitwuzla baseline, hostile RTL isolation, and macOS, Linux,
and Windows downstream API matrix. This closes every predeclared v5 gate. It
does not establish algorithmic novelty, product validity, or production
readiness.
