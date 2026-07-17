# CQ-SAT/GCC design-partner brief

## Sendable introduction

**Subject:** Design-partner evaluation of bounded RTL configuration safety

> I am inviting embedded-product teams to evaluate
> CQ-SAT/GCC, an open-source bounded verification tool for configuration and
> control-safety properties expressed against RTL.
>
> Repository and releases:
> https://github.com/kabudu/continuation-quotient-sat
>
> CQ-SAT/GCC records the exact source/configuration snapshot, assumptions,
> property boundary, horizon, result, and replay evidence. A SAFE result means
> only that no declared bad output was found through that reviewed bounded
> horizon; an UNSAFE result includes a replayable named counterexample.
>
> The current research preview has a stable CLI and evidence schema, independent
> public oracle comparisons, bounded Linux execution, a hostile-input isolation
> profile, and a machine-enforced external-evidence protocol. I am seeking teams
> willing to run self-directed evaluations against representative, non-public
> RTL requirements and compare results with an independently owned oracle or
> pre-existing reviewed result.
>
> The preferred model keeps confidential RTL on a partner-owned ephemeral Linux
> worker. No RTL, property text, trace, credential, or partner identity needs to
> enter the public repository. Only agreed aggregate counts and limitations need
> be publishable.
>
> CQ-SAT/GCC is not certified, production-qualified, or a replacement for your
> safety lifecycle. This pilot exists to determine whether the evidence is
> correct, reproducible, operationally usable, and suitable as one bounded input
> to an independently reviewed assurance case.
>
> If this is relevant, please reply with a non-confidential description of your
> product/control domain, RTL toolchain, preferred worker ownership, whether an
> independent expected result is available, and the appropriate contact for the
> final outcome. I do not plan to participate in setup or execution, access your
> RTL, or review private intermediate evidence. The repository contains the
> operating, security, intake, review, and reporting guidance needed to conduct
> the evaluation directly. Please do not send source code or other confidential
> data to me or through public project channels.

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

## Suitable partners

The strongest pilot partners have:

- an embedded, FPGA, ASIC, control, or device team with reviewable RTL;
- concrete safety or configuration requirements that can be represented as
  named bad outputs under explicit assumptions and a bounded horizon;
- known SAFE, known UNSAFE, and expected rejection or unsupported-input cases;
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
3. Select one immutable CQ release and record its annotated tag and commit.
4. Qualify the partner-owned ephemeral Linux worker from the published runbook.
5. Map each requirement to reviewed sources, top, parameters, assumptions, bad
   outputs, reset/startup policy, and bounded horizon.
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
- the self-service design-partner brief, private intake, assessor scope, outcome
  template, external-evidence register, and executable production gate; and
- a private GitHub Security Advisory route for suspected repository
  vulnerabilities, without any commitment to partner-specific evaluation work.

## What the partner provides

- authorised use of representative requirements and RTL on the agreed worker;
- requirement, model-boundary, assumption, horizon, and expected-result review;
- an independent oracle or pre-existing reviewed expected result;
- operator time for qualification, execution, replay, repetition, and runbook
  exercises;
- attributable private approval or rejection of each evidence row; and
- permission for an agreed, redacted aggregate report—or an explicit decision
  that the pilot cannot count toward the public production cohort.
- delivery to the maintainer of only the final non-confidential outcome and
  suitability report required by `OUTCOME_REPORT_TEMPLATE.md`.

## Go/no-go call

Before any confidential transfer, confirm:

- the use case is bounded RTL verification rather than a request for whole-device
  certification or an unbounded proof;
- all parties accept CQ-SAT/GCC's research-preview status and claim boundary;
- the partner's data-handling and incident contacts are authorised;
- an independent expected result is feasible;
- unresolved disagreements and failures will be retained; and
- the partner can complete an attributable private review record.
- the partner accepts that the maintainer will not operate, supervise, or access
  the private evaluation.

If any item is false, pause the pilot or treat it as non-qualifying exploration.
