# Confidential design-partner pilot intake

Copy this template into the partner's approved access-controlled system. Do not
complete it in a public GitHub issue, pull request, discussion, or repository.
Use opaque IDs in the public evidence register and keep the mapping private.
The partner and its independent reviewer operate the evaluation without
maintainer access. Send the maintainer only the final non-confidential outcome
defined in `OUTCOME_REPORT_TEMPLATE.md`.

## Authority and contacts

- Private engagement ID:
- Partner legal entity:
- Product/data owner and approval reference:
- Technical owner:
- Safety/requirements reviewer:
- Security and incident contact:
- Independent oracle owner:
- Independent aggregate reviewer:
- Permitted operators:

## Data-handling agreement

- Execution location and owner:
- Permitted source/data classifications:
- Prohibited data and credentials:
- Authorised access identities:
- Transfer mechanism, if any:
- Encryption requirements:
- Retention period and deletion evidence:
- Backup policy:
- Incident notification route and deadline:
- Permitted private report recipients:
- Permitted public aggregate fields:
- Identity/redaction constraints:
- Applicable NDA, DPA, export, regulatory, or contractual references:

This section records an agreement; it is not a substitute for legal, privacy,
export-control, or regulatory advice.

## Immutable evaluation target

- CQ release tag:
- CQ commit (full object ID):
- Artifact schema and firmware CLI versions:
- Isolation-profile version and image digest:
- Rust, Yosys, SymbiYosys, Z3, and operating-system versions:
- Worker opaque ID:
- Qualification report reference and digest:
- Rollback target:

## Project inventory

Repeat for each project:

- Private project ID / public opaque project ID:
- Product or control domain / public opaque domain ID:
- Source/toolchain provenance:
- Top-level design and hierarchy characteristics:
- Includes, parameters, memories, and reset/startup characteristics:
- Confidentiality classification:
- Independent expected-result method:
- Planned SAFE / UNSAFE / FAILURE configuration counts:
- Known unsupported constructs or exclusions:

## Configuration record

Repeat for every configuration before execution:

- Private and public opaque record IDs:
- Requirement ID, owner, text location, and requirement-record digest:
- Input snapshot location and digest:
- Sources, top, include directories, and parameter overrides:
- Environment assumptions and owner:
- Named bad outputs and rationale:
- Reset/startup policy:
- Bounded horizon and adequacy rationale:
- Expected result (`SAFE`, `UNSAFE`, or `FAILURE`):
- Independent expected-result source and date:
- Acceptance and escalation criteria:

Complete after execution:

- CQ result and exit class:
- Oracle result:
- Runtime and peak memory:
- Validated bundle digest, or reason no bundle exists:
- Isolation-report digest, or reason no report exists:
- Repeated result and repeated-bundle validation:
- UNSAFE witness replay result and engineering triage:
- Disagreement/failure investigation and disposition:
- Reviewer decision (`reviewed` or `rejected`):
- Private report reference:

No configuration may disappear because it timed out, crashed, was unsupported,
or disagreed. It remains in the private record and aggregate denominator.

## Operator exercises

- [ ] Fresh installation and qualification
- [ ] SAFE, UNSAFE, and tool/input-failure exit handling
- [ ] Evidence validation and separately retained digests
- [ ] Upgrade qualification and rollback
- [ ] Timeout, resource-limit, and interrupted-run handling
- [ ] Incident escalation and confidential artifact handling
- [ ] Backup, restoration, retention, and deletion evidence

## Closeout approvals

- All attempted configurations accounted for:
- All disagreements reconciled or explicitly open:
- All UNSAFE witnesses replayed and triaged:
- All retained bundles validated and repeated:
- Critical/high security findings open:
- Partner technical approval and date:
- Partner safety/requirements approval and date:
- Data-owner publication/redaction approval and date:
- Independent reviewer conclusion and date:
- Evidence-register export reference and digest:
- Aggregate report reference and digest:
- Final non-confidential outcome report reference:
- Confirmation that no private source, trace, credential, identity mapping, or
  intermediate record is being sent to the maintainer:

## Independent aggregate attestation

After all private approvals, the independent aggregate reviewer creates an
LF-only file with exactly these 16 keys. Replace bracketed values; do not quote
values or include confidential names, formulas, commas, or private paths.

```text
protocol_version=2
target_tag=[ANNOTATED_RELEASE_TAG]
target_commit=[FULL_40_CHARACTER_COMMIT]
register_digest=sha256:[REGISTER_SHA256]
security_assessment_status=PASS
security_assessment_report=[IMMUTABLE_REPORT_REFERENCE]
technical_review_status=PASS
technical_review_report=[IMMUTABLE_REPORT_REFERENCE]
operator_exercises_status=PASS
data_handling_status=PASS
independent_reviewer_id=[OPAQUE_REVIEWER_ID]
independent_aggregate_status=PASS
independent_aggregate_report=[IMMUTABLE_REPORT_REFERENCE]
critical_findings_open=0
high_findings_open=0
assessment_date=[YYYY-MM-DD]
```

The reviewer signs the exact attestation bytes with their approved OpenSSH key:

```sh
ssh-keygen -Y sign -f REVIEWER_PRIVATE_KEY \
  -n gcc-production-evidence-v2 ATTESTATION.conf
```

The release authority prepares `ALLOWED_SIGNERS` independently from the
submitted evidence. It maps the opaque reviewer ID to the previously approved
public key and restricts it to namespace `gcc-production-evidence-v2`. Never
accept an allowed-signers file supplied only alongside the evidence it is meant
to authenticate.

Run the final gate from the reviewed source repository:

```sh
scripts/external-evidence-register-check.sh \
  --production-gate REGISTER.csv ATTESTATION.conf ALLOWED_SIGNERS \
  ATTESTATION.conf.sig REVIEWED_SOURCE_REPOSITORY
```

Exit 0 is necessary but not sufficient for a public production claim: the
independent approver must also confirm that no attempted configuration or
material limitation was omitted from the register and reports.
