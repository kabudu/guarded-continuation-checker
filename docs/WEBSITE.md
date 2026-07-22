# Guarded Continuation Checker website brief

## Recommendation

Create a small, static site at `guardedcontinuation.org`. The site should be the
human-readable front door for evaluators, while GitHub remains the canonical
source for code, releases, technical documentation, issues, and security
reporting.

Do not register or publish the domain until availability, name clearance, and
the canonical repository rename have been checked. Do not present the project
as certified or production-qualified while the corresponding gates remain open.

## Audiences and journeys

1. **Firmware and RTL leads:** understand the bounded problem, inspect a
   product-shaped example, and reach the evaluation guide.
2. **Verification engineers:** inspect supported inputs, proof evidence, exact
   fallback, limitations, benchmarks, and reproducibility material.
3. **Safety and assurance reviewers:** find the claim boundary, evidence schema,
   independent-oracle protocol, standards-applicability notes, and production
   gap register.
4. **Design partners:** follow a self-service evaluation path and return only a
   non-confidential outcome and suitability assessment.

## Initial site map

- `/`: bounded value proposition, qualification, architecture, evidence, and
  two clear actions: inspect the evidence or run an evaluation;
- `/how-it-works`: Guarded Continuation Checker workflow and CQ-SAT backend;
- `/use-cases`: embedded firmware and RTL examples, including the infusion-pump
  configuration-safety example;
- `/evidence`: independent baselines, public corpus, certificates,
  reproducibility, negative results, and open production gaps;
- `/evaluate`: self-service design-partner brief, runbook, templates, and
  outcome route;
- `/docs`: curated links into versioned repository documentation;
- `/releases`: links to signed/tagged GitHub releases and changelog; and
- `/about`: individual-maintainer status, licence, citation, and contact route.

Security reports must continue through private GitHub Security Advisories. The
site must not collect partner RTL, properties, traces, credentials, identity
mappings, or confidential assessment evidence.

## Homepage claim hierarchy

1. **Guarded Continuation Checker, powered by CQ-SAT**
2. **An evaluation-ready, proof-carrying bounded verification platform for
   embedded firmware and RTL.**
3. **Evaluation-ready research prototype. Not certified or
   production-qualified; a SAFE result is bounded by the reviewed model,
   assumptions, property, and horizon.**

Every result example must name its boundary and distinguish verified evidence
from whole-device safety. Measurements should include fallbacks and negative
results where material.

## Delivery approach

Use a static, accessibility-first implementation with no account system,
database, analytics dependency, or mutable copy of release data in the first
version. Keep site content beside the source repository or in a dedicated
website repository once its deployment lifecycle diverges. Automate link,
accessibility, spelling, and build checks. Pin release links to GitHub and show
the exact version wherever behaviour or schema is described.

After the first production release passes every applicable gate, reuse the
repository's canonical, accessible SVG architecture diagram on the website.
Embed the same optimised asset in the project README so the platform boundary,
CQ-SAT engine, verification path, proof artifacts, and integration paths cannot
drift between the two surfaces.

The first public version is ready only when:

- the repository rename and redirects are verified;
- the product name, claim boundary, and release version agree across the site,
  repository, citation, and partner material;
- all calls to action reach public, versioned resources;
- the design-partner path remains entirely self-service;
- the brand assets render correctly at desktop, mobile, social-card, monochrome,
  and 16–24 px favicon sizes; and
- the site contains no unsupported novelty, certification, production-readiness,
  or whole-device-safety claim.
