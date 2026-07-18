# Guarded Continuation Checker brand system

## Brand architecture

**Guarded Continuation Checker** is the umbrella product and public project.
The preferred signature is:

> Guarded Continuation Checker, powered by CQ-SAT

**CQ-SAT** is the exact continuation-quotient engine and specialised backend.
The Rust package and executable are `guarded-continuation-checker`; Rust imports
use `guarded_continuation_checker`. The former pre-release research name has no
compatibility guarantee and must not appear in current commands, packages or
integration guidance. Do not shorten the executable to `gcc`.

The initials **GCC** may be used only after the full product name appears in the
same document or page. Bare “GCC” is already overwhelmingly associated with the
GNU Compiler Collection in the same engineering market. External headlines,
search metadata, package names, commands and domains should lead with “Guarded
Continuation,” not the initials.

Before this architecture, repository material sometimes expanded GCC as
“Global Checkpoint Clauses” inside `CQ-SAT/GCC`. That phrase may remain in
historical result descriptions, but new product language reserves GCC for
Guarded Continuation Checker and describes checkpoint clauses without an
acronym.

## Positioning

Canonical short description:

> An evaluation-ready, proof-carrying bounded verification platform for
> embedded firmware and RTL, powered by CQ-SAT.

Required qualification while production gates remain open:

> Guarded Continuation Checker is an evaluation-ready research prototype. It
> is not certified, production-qualified, or evidence that an entire device is
> safe.

The first sentence says what the platform does. The qualification says what the
current evidence does not support. They should appear together in partner,
release and website material.

## Visual idea

The mark is a **guard aperture**:

- three paths represent alternative Boolean continuations;
- exact convergence represents quotienting equivalent residual behaviour;
- the ring represents an independently checked obligation; and
- the open frame and exiting arrow represent a validated continuation rather
  than a closed shield or generic security lock.

The mark must not use animals, gnu imagery, compiler motifs, orange/brown GNU
colours, padlocks, circuit-board decoration, or a standalone “GCC” monogram.

## Assets

- [`mark.svg`](../assets/brand/mark.svg): primary symbol on light backgrounds;
- [`mark-mono.svg`](../assets/brand/mark-mono.svg): one-colour and small-format
  reproduction; and
- [`logo-horizontal.svg`](../assets/brand/logo-horizontal.svg): default project,
  repository and website signature; and
- [`social-card.svg`](../assets/brand/social-card.svg): repository, release and
  website social-preview source.

Keep clear space around the mark equal to the diameter of its centre aperture.
Do not rotate, stretch, recolour individual paths, add effects, or place the
full-colour mark on visually noisy backgrounds. At sizes below 24 CSS pixels,
use the monochrome mark.

## Colour

| Token | Hex | Use |
|---|---|---|
| Continuation navy | `#081F3D` | wordmark, frame, primary text |
| Verification teal | `#0A9A92` | checked aperture, primary action |
| Signal cyan | `#08A9F0` | active path and data accent |
| Branch violet | `#6956D9` | alternative-path accent |
| Slate | `#24415E` | secondary text |
| Mist | `#F4F8FA` | quiet panels and technical diagrams |
| White | `#FFFFFF` | primary background and aperture interior |

Use navy on white for long text. Teal is an accent, not body text. Safety
results must not rely on colour alone; always pair colour with `SAFE`, `UNSAFE`,
`VERIFIED`, or another explicit label.

## Typography

Use Inter for product and website interfaces where it can be served or bundled
lawfully, falling back to the system sans-serif stack. Use the system monospace
stack for commands, evidence fields, hashes and `CQ-SAT`. Documentation source
must remain readable without web fonts.

## Voice

The voice is precise, calm and evidence-led:

- lead with the bounded outcome and its scope;
- distinguish a verified result from operational success;
- say “I” for maintainer outreach, never an invented corporate “we”;
- prefer “evaluation-ready research prototype” to “product” where readiness is
  material;
- never claim certification, whole-device safety, general SAT superiority,
  scholarly novelty, or production readiness before the corresponding gate is
  independently closed; and
- report negative measurements and fallbacks with the positive evidence.

## Repository and web naming

The intended repository name is `guarded-continuation-checker`. GitHub redirects
the previous repository URL after rename; documentation should nevertheless use
the canonical new URL once the rename is complete.

`guardedcontinuation.org` is the preferred website candidate because it leads
with the distinctive full name. Domain availability and trademark/name
clearance must be checked before registration. A future website should use
GitHub as the source and release authority rather than duplicating mutable
technical truth. See the [website brief](WEBSITE.md) for the proposed information
architecture and publication gates.
