# BTOR2 source-separated component cohort v1

This eight-row answer-balanced cohort compares a source-separated controller,
plant, and wiring contract against the equivalent monolithic product. It also
includes a semi-implicit plant that the phase specialisation must reject.

| Case | Horizon | Answer | Component backend | Monolithic backend | Explicit bytes | Component bytes | Monolithic portfolio bytes |
|---|---:|---|---|---|---:|---:|---:|
| Braking base | 255 | SAFE | phase-contract | braking-phases | 1,180,313 | 494 | 386 |
| Braking base | 256 | UNSAFE | composed-search | explicit-search | 473 | 901 | 473 |
| Reused controller, fast plant | 127 | SAFE | phase-contract | braking-phases | 287,786 | 493 | 385 |
| Reused controller, fast plant | 128 | UNSAFE | composed-search | explicit-search | 345 | 645 | 345 |
| Motor stop | 159 | SAFE | phase-contract | braking-phases | 453,342 | 494 | 386 |
| Motor stop | 160 | UNSAFE | composed-search | explicit-search | 377 | 709 | 377 |
| Semi-implicit control | 127 | SAFE | composed-search | explicit-search | 283,227 | 181,238 | 283,227 |
| Semi-implicit control | 128 | UNSAFE | composed-search | explicit-search | 345 | 645 | 345 |

Every answer agrees and both certificate families verify against the original
separate sources. One controller source is reused unchanged with two plant
sources. The admitted SAFE component artifacts are more than 99.8% smaller
than explicit monolithic layers, but 108 bytes larger than the existing
monolithic specialised certificate. Unsafe component witnesses are also larger
because they bind three source artifacts. The semi-implicit SAFE composed
search artifact is 36.01% smaller than the monolithic explicit encoding because
the controller and plant state vectors remain separated.

This is positive modularity and source-integrity evidence, not a performance or
novelty breakthrough over the existing monolithic specialisation.
