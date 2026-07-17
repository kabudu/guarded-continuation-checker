# CQ-SAT/GCC design-partner brief

## Sendable introduction

**Subject:** Design-partner evaluation of bounded RTL configuration safety

> We are inviting a small number of embedded-product teams to evaluate
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
> profile, and a machine-enforced external-evidence protocol. We are now seeking
> teams willing to run controlled evaluations against representative, non-public
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
> independent expected result is available, and the appropriate technical and
> security contacts. Please do not send source code or other confidential data
> in the initial response.

Replace “we” and contact details for the sender, but do not weaken or remove the
bounded-result and non-certification wording.

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

1. Exchange only non-confidential scope and identify authorised technical,
   security, safety, and data-owning contacts.
2. Agree data location, access, retention, deletion, incident handling,
   report-redaction rights, commercial terms, and any confidentiality agreement
   outside this repository. This document is not legal advice or an NDA.
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
10. Obtain independent review of the private records and aggregate conclusion.

## What the project provides

- source and tagged releases under the repository licence;
- versioned CLI, schema validator, isolation wrapper, operations runbook, public
  corpus, independent-oracle examples, and production-gate checker;
- engineering support scoped and agreed separately for setup, failure triage,
  and reproducibility; and
- prompt private handling of suspected vulnerabilities through GitHub Security
  Advisories.

## What the partner provides

- authorised use of representative requirements and RTL on the agreed worker;
- requirement, model-boundary, assumption, horizon, and expected-result review;
- an independent oracle or pre-existing reviewed expected result;
- operator time for qualification, execution, replay, repetition, and runbook
  exercises;
- attributable private approval or rejection of each evidence row; and
- permission for an agreed, redacted aggregate report—or an explicit decision
  that the pilot cannot count toward the public production cohort.

## Go/no-go call

Before any confidential transfer, confirm:

- the use case is bounded RTL verification rather than a request for whole-device
  certification or an unbounded proof;
- all parties accept CQ-SAT/GCC's research-preview status and claim boundary;
- data-handling and incident contacts are authorised;
- an independent expected result is feasible;
- unresolved disagreements and failures will be retained; and
- the partner can complete an attributable private review record.

If any item is false, pause the pilot or treat it as non-qualifying exploration.
