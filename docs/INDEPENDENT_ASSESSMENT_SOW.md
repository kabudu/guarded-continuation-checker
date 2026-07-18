# Independent assessment statement of work

This is a technical scope template for requesting proposals. Commercial terms,
liability, intellectual property, confidentiality, data protection, export
controls, and governing law require separate authorised agreements. This
template is not legal advice and does not ask the assessor to certify a product
or declare conformity with a safety standard.

## Sendable request

**Subject:** Independent security and formal-verification assessment request

> I am seeking an independent assessment of Guarded Continuation Checker,
> powered by CQ-SAT, an open-source bounded
> RTL configuration-safety verification tool. The target will be one immutable,
> annotated release tag. The engagement covers both its documented Linux threat
> model/isolation boundary and the correctness of its RTL-to-bounded-result and
> evidence semantics.
>
> I require hands-on reproduction and adversarial testing, attributable reports,
> finding reproduction steps, and independent retesting after remediation. CI,
> automated scanning, or a testimonial alone will not satisfy the engagement.
>
> Please respond with relevant Linux container, Rust/parser, supply-chain,
> SAT/BMC, RTL synthesis, or embedded assurance experience; proposed personnel;
> independence/conflict declaration; methodology; environment needs; schedule;
> deliverables; handling of confidential findings; retest terms; and commercial
> proposal. The evaluation is self-directed: I do not plan to participate in its
> operation or receive private source, traces, credentials, or intermediate
> records. Please use the repository guidance directly and send me only the final
> non-confidential outcome, suitability conclusion, material limitations, gate
> result, and agreed public report references. Do not send sensitive customer
> information in the initial response or final outcome.

## Target and independence

- Target tag: `TARGET_TAG`
- Target commit: `TARGET_COMMIT`
- Required source: annotated tag that peels to the target commit
- Security assessor(s): `ASSESSOR`
- Technical reviewer(s): `REVIEWER`
- Conflicts, prior contributions, financial interests, and dependencies:
  `DECLARATION`

Reviewers must not have authored or approved the implementation they assess and
must be free to report adverse findings. One supplier may perform both workstreams
only when named personnel demonstrate both competencies and issue distinct,
attributable conclusions.

## Security workstream

Perform the full security scope in `EXTERNAL_EVIDENCE_PROTOCOL.md`, including
hands-on malformed-input, filesystem/path, special-file, resource-exhaustion,
process-tree, interruption, container-control, artifact-confusion/tampering,
supply-chain, secret, untrusted-fork, confidentiality, and denial-of-service
testing.

Reproduce at least seven attributable security cases, including:

1. persistent malformed-input mutation corpus;
2. SAFE and UNSAFE hostile-RTL isolation paths;
3. watchdog deadline and active-container cleanup;
4. interruption/signal cleanup;
5. output overlap and pre-existing-output rejection;
6. symlink/special-path rejection; and
7. artifact tampering and result-status disagreement rejection.

The assessor should add attacks based on independent judgement. Passing the
minimum list is not a finding-free conclusion.

## Technical workstream

Review synthesis and model semantics, bounded-result meaning, assumptions,
startup/reset policy, named traces, all exact backend and fallback paths,
witness reconstruction/replay, refusal/failure semantics, independent-oracle
ownership, CLI/schema compatibility, reproducibility, and external claim bounds.

Independently reproduce the public corpus and supply at least three additional
adversarial models not authored by the maintainer: one expected SAFE, one
expected UNSAFE, and one expected tool/input failure. Record exact inputs,
environments, expected-result ownership, CQ/oracle results, exit classes,
evidence validation, replay, resource measurements, and limitations.

## Required deliverables

1. Independence and competency declaration.
2. Exact target/environment inventory and tested commands.
3. Separate attributable security and technical reports.
4. Machine-readable external-evidence register rows conforming to protocol v1.
5. Findings with assessor-selected severity method, impact, evidence, and
   reproduction steps.
6. Explicit coverage gaps and limitations.
7. Remediation verification for every critical/high finding and disposition of
   all other findings.
8. Signed or otherwise attributable final conclusions suitable for reference by
   the independent aggregate reviewer.

Reports may be confidential during coordinated remediation. Suspected
vulnerabilities must use the repository's private GitHub Security Advisory route,
not a public issue. The final public disclosure scope and timing require explicit
agreement and must not expose partner RTL, traces, credentials, or identities.
The partner and assessor retain private evaluation records under their own
authorised arrangements; the maintainer receives only the final redacted outcome
defined by `OUTCOME_REPORT_TEMPLATE.md`.

## Acceptance

The engagement is accepted only when:

- target tag/commit and environments are unambiguous and reproducible;
- all required cases have inspectable evidence;
- no critical/high finding remains open;
- other findings have owners and dispositions;
- corrections are independently retested;
- reports state that bounded verification evidence is not whole-product safety,
  certification, tool qualification, or unbounded proof; and
- the v0.20-or-later production-gate checker accepts the attributable package
  only after the separate partner cohort is also complete.

The maintainer's agreement with a report does not make it independent; the named
reviewer remains responsible for their methods, evidence, and conclusion.
