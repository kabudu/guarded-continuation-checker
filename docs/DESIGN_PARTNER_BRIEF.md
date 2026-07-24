# Guarded Continuation Checker design-partner brief

## Sendable introduction

**Subject:** Independent evaluation of proof-carrying firmware and RTL verification

> I am inviting embedded-product, semiconductor, verification-service, and
> verification-tool teams to evaluate Guarded Continuation Checker, powered by
> CQ-SAT: an open-source, proof-carrying bounded verification platform for
> embedded firmware and RTL.
>
> Repository and releases:
> https://github.com/kabudu/guarded-continuation-checker
>
> The recommended evaluation target is immutable release v0.30.0. Guarded
> Continuation Checker records the exact source and configuration snapshot,
> assumptions, property boundary, horizon, result, and replay evidence. It can
> also compare related design revisions and distinguish source changes that
> invalidate prior evidence from changes that actually alter a bounded SAFE or
> UNSAFE result. A SAFE result means only that no declared bad output was found
> through that reviewed bounded horizon; an UNSAFE result includes a replayable
> named counterexample.
>
> Guarded Continuation Checker is an evaluation-ready research prototype with a
> stable CLI and evidence schema, independent public oracle comparisons,
> bounded Linux execution, a hostile-input isolation profile, and a machine-enforced
> external-evidence protocol. I am seeking teams willing to independently choose
> representative designs and RTL requirements, run the evaluation using the
> repository guidance, and compare results with an independently owned oracle or
> pre-existing reviewed result.
>
> The preferred model keeps confidential RTL on a partner-owned ephemeral Linux
> worker. No RTL, property text, trace, credential, or partner identity needs to
> enter the public repository. Only agreed aggregate counts and limitations need
> be publishable.
>
> Guarded Continuation Checker is not certified, production-qualified, or a
> replacement for your safety lifecycle. This pilot exists to determine whether
> the evidence is correct, reproducible, operationally usable, and suitable as
> one bounded input to an independently reviewed assurance case.
>
> If this is relevant, your team can use the repository resources directly: it
> chooses the designs or revisions, requirements, worker, oracle, and independent
> assessor, then conducts the evaluation without my involvement. Verification
> vendors may use an existing product as the independent oracle and assess
> whether GCC's deterministic bundles, replay evidence, and revision-impact
> certificates could complement their platform or assurance services. I do not
> plan to participate in setup or execution, access your RTL, or review private
> intermediate evidence. Please send me only the completed non-confidential
> outcome and suitability report, including agreed aggregate results, material
> limitations, the production-gate result, and any agreed public report
> references. Please do not send source code, property text, traces, credentials,
> partner identity mappings, or other confidential data to me or through public
> project channels. A reproducible, non-confidential repository defect may be
> reported through a normal public issue, and a suspected vulnerability through
> a private GitHub Security Advisory.

Add the sender's contact details, but do not weaken or remove the bounded-result,
self-service, confidentiality, or non-certification wording.

## Self-service operating model

The design partner owns the evaluation from intake through independent review.
The individual maintainer:

- supplies the public repository and tagged releases as-is under the licence;
- does not need access to partner RTL, properties, traces, workers, credentials,
  private registers, meetings, or intermediate findings;
- does not operate or supervise the partner's evaluation;
- does not promise engineering, commercial, legal, safety, or certification
  support; and
- is told only the final suitability outcome, non-confidential aggregate counts,
  material limitations, production-gate result, and public report references.

The partner may open a normal public issue for a reproducible, non-confidential
repository defect. Suspected vulnerabilities must use GitHub Security Advisories.
Neither route should contain partner source, property text, traces, credentials,
identity mappings, or other confidential material.

## Evaluation tracks

Partners may complete one or more tracks.

### Bounded design evaluation

Use one reviewed design, explicit assumptions, named bad outputs, and a bounded
horizon. Include known SAFE, known UNSAFE, expected rejection, and unsupported
input cases where available. Compare each GCC result with an independently owned
oracle or a pre-existing reviewed result, replay every counterexample, and retain
every disagreement.

### Revision-impact evaluation

Use related old and new source revisions with a reviewed retained environment.
Include independent changes and combination-only regressions where available.
Assess whether GCC's source-bound certificate correctly identifies:

- source or configuration drift that invalidates retained evidence;
- changes that do not alter the bounded answer;
- inclusion-minimal changes that alter a bounded SAFE or UNSAFE answer; and
- the earliest observed bounded failure and its replayable witness.

The v0.30.0 public cohort contains 20 connected OpenTitan PWM observations and
independent agreement with maintained Yosys, rIC3, and Certifaiger evidence. The
published matched-workflow timing and memory measurements are cohort-specific,
not universal solver-performance claims. Revision-impact analysis remains a
research surface and is not included in the frozen `production-firmware`
support profile.

### Verification-platform interoperability

A verification-tool or verification-service partner may use its existing
product or reviewed process as the independent oracle. In addition to result
agreement, assess whether GCC's evidence schema, replay workflow, exact source
binding, and revision-impact certificate could be consumed by or complement the
partner's existing platform. This track is an interoperability and suitability
evaluation, not a request to replace an established verification engine,
certification process, or sign-off authority.

## Suitable partners

The strongest pilot partners have:

- either an embedded, FPGA, ASIC, control, or device team with reviewable RTL,
  or a verification product, service, or independent assessment practice able
  to compare GCC with an established workflow;
- concrete safety or configuration requirements that can be represented as
  named bad outputs under explicit assumptions and a bounded horizon;
- known SAFE, known UNSAFE, and expected rejection or unsupported-input cases;
- for revision-impact work, related source revisions with reviewed expected
  effects, including combination-only regressions where available;
- an independent oracle, prior reviewed outcome, or personnel able to establish
  one without reusing CQ implementation logic;
- an ephemeral Linux worker and an operator able to follow the runbook; and
- authority to retain attributable private evidence while publishing agreed
  aggregate, non-confidential results.

The full production cohort requires at least two organisations, three projects,
two domains, two workers, and 30 partner configurations. A prospective partner
does not need to supply the whole cohort; a useful target is approximately 15
reviewed configurations, subject to the partner's actual projects and approval.

## Engagement sequence

1. The partner identifies authorised technical, security, safety, data-owning,
   oracle, and independent-review contacts without involving the maintainer.
2. The partner establishes data location, access, retention, deletion, incident
   handling, report-redaction rights, commercial terms, and any confidentiality
   agreement with its chosen assessor. This document is not legal advice or an
   NDA, and the maintainer is not a recipient of partner source data.
3. Select immutable Guarded Continuation Checker release v0.30.0 and record its
   annotated tag and commit.
4. Qualify the partner-owned ephemeral Linux worker from the published runbook.
5. Select the bounded-design, revision-impact, or interoperability track. Map
   each requirement to reviewed sources, revisions, top, parameters,
   assumptions, bad outputs, reset/startup policy, and bounded horizon.
6. Record the independent expected-result source before running CQ.
7. Run through the hostile-RTL isolation profile, validate every retained
   bundle, replay and triage every UNSAFE trace, and repeat each result.
8. Complete the operator exercises and populate the external evidence register
   using opaque public identifiers.
9. Reconcile every disagreement without dropping it from the denominator.
10. Obtain independent review, then send the maintainer only the completed
    non-confidential outcome report and any agreed public evidence references.

## Repository resources

- source and tagged releases under the repository licence;
- versioned CLI, schema validator, isolation wrapper, operations runbook, public
  corpus, independent-oracle examples, and production-gate checker;
- v0.30.0 revision-impact certificate commands, a public 20-observation
  OpenTitan PWM cohort, and maintained Yosys, rIC3, and Certifaiger comparisons;
- the self-service design-partner brief, private intake, assessor scope, outcome
  template, external-evidence register, and executable production gate; and
- a private GitHub Security Advisory route for suspected repository
  vulnerabilities, without any commitment to partner-specific evaluation work.

## What the partner provides

- authorised use of representative requirements and RTL on the agreed worker;
- requirement, revision, model-boundary, assumption, horizon, and
  expected-result review;
- an independent oracle or pre-existing reviewed expected result;
- operator time for qualification, execution, replay, repetition, and runbook
  exercises;
- attributable private approval or rejection of each evidence row; and
- permission for an agreed, redacted aggregate report, or an explicit decision
  that the pilot cannot count toward the public production cohort; and
- delivery to the maintainer of only the final non-confidential outcome and
  suitability report required by `OUTCOME_REPORT_TEMPLATE.md`.

## Go/no-go call

Before any confidential transfer, confirm:

- the use case is bounded RTL verification rather than a request for whole-device
  certification or an unbounded proof;
- all parties accept Guarded Continuation Checker's evaluation-ready
  research-prototype status and claim boundary;
- the partner's data-handling and incident contacts are authorised;
- an independent expected result is feasible;
- unresolved disagreements and failures will be retained;
- the partner can complete an attributable private review record; and
- the partner accepts that the maintainer will not operate, supervise, or access
  the private evaluation.

If any item is false, pause the pilot or treat it as non-qualifying exploration.
