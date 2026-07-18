# Standards applicability and assurance claims

Guarded Continuation Checker is a verification-support tool, not a certified
product, accredited laboratory, safety lifecycle, or substitute for engineering judgement. This
document bounds the evidence that an adopter may place in a product assurance
case. It does not declare conformity with any standard.

The product owner remains responsible for selecting the applicable standards,
editions, national adoptions, regulatory guidance, safety classification, tool
confidence or qualification route, independence, and acceptance criteria. That
assessment must use licensed normative texts and competent safety personnel.

## Exact claim boundary

For an admitted project configuration, Guarded Continuation Checker can claim
only that:

- the recorded RTL snapshot was synthesized with the recorded configuration;
- under the recorded constant environment assumptions, no declared bad output
  was found through the recorded bounded horizon (`SAFE`), or a named bounded
  counterexample was found (`UNSAFE`);
- the result and its inputs are bound by the versioned artifact manifest; and
- the bundle passes the matching executable schema validator.

`SAFE` is therefore **bounded, model-relative evidence**. It is not a claim that
the physical product is safe, that the requirement is complete, or that the bad
state is unreachable beyond the analysed horizon. `UNSAFE` is evidence of a
modelled counterexample, which still requires engineering triage against the
source requirement and implementation context.

Guarded Continuation Checker does not analyse analogue behaviour, mechanical
failure, random hardware failure rates, real-time schedulability, compiler correctness, human
factors, manufacturing variation, cybersecurity of the target product, or the
completeness of hazards and safety requirements. It does not establish
equivalence between RTL and a fabricated device or deployed firmware image.

## Applicability matrix

| Framework | Where tool evidence may assist | What tool evidence does not establish |
| --- | --- | --- |
| ISO 26262:2018, road-vehicle functional safety | Bounded verification evidence for a precisely modelled E/E hardware or software safety requirement; reproducible support for review, configuration control, and change-impact workflows. | ASIL determination, hazard analysis and risk assessment, safety-plan completeness, independence, hardware metrics, production controls, tool qualification, or ISO 26262 conformity. |
| IEC 61508:2010, generic E/E/PE functional safety | Bounded formal evidence for a declared safety property in an electrical/electronic/programmable electronic subsystem, retained within the adopter's safety lifecycle. | SIL allocation or achievement, systematic capability, random-failure integrity, lifecycle compliance, functional-safety assessment, tool qualification, or certification. |
| IEC 62304:2006+A1:2015, medical-device software lifecycle | Verification evidence linked by the manufacturer to a software or embedded-device requirement, anomaly investigation, configuration management, and maintenance records. | Software safety classification, risk management, lifecycle-process compliance, device validation or final release, clinical safety, regulatory clearance, or tool qualification. |
| FDA infusion-pump lifecycle guidance | A bounded check of a specific, modelled configuration interlock or control-state requirement, plus a replayable counterexample for design verification. | Verification or validation of the complete pump, dose accuracy, alarm performance, human factors, reliability, clinical use, manufacturing, premarket acceptance, or FDA clearance. |
| IEC 81001-5-1:2021, health-software security lifecycle | The isolated evaluation profile and evidence-integrity controls may support the adopter's secure development tooling records. | Cybersecurity of the target health software, secure-lifecycle conformity, threat coverage, penetration testing, vulnerability management, or regulatory acceptance. |

These mappings are intentionally at framework level. Clause-level credit must be
decided by the adopter from the applicable normative edition; Guarded
Continuation Checker must not ship a universal clause-compliance table because project use, safety class,
jurisdiction, and tool role change the required evidence.

## Evidence-package responsibilities

Before using a Guarded Continuation Checker bundle in an assurance case, the
adopter must retain a review
record containing:

1. the uniquely identified safety or verification requirement and its owner;
2. the rationale that the RTL sources, top, parameters, assumptions, bad
   outputs, reset policy, and horizon represent that requirement;
3. the exact CQ release, toolchain revisions, immutable isolation image, command
   configuration, and independently retained bundle/report digests;
4. the validated bundle, and counterexample triage for every `UNSAFE` result;
5. independent review appropriate to the product's safety classification;
6. change-impact rules that invalidate or repeat the analysis when any input,
   requirement, tool, environment, or acceptance criterion changes; and
7. the adopter's documented tool-confidence or qualification decision.

A CI green check alone is not this record. A team must not transform `SAFE` into
an unconditional build-release decision unless its approved lifecycle defines
the bounded property, horizon, independent checks, and escalation path.

## Permitted and prohibited wording

Acceptable:

- “Guarded Continuation Checker produced validated bounded verification
  evidence for requirement `REQ-ID` over the reviewed model and horizon.”
- “Guarded Continuation Checker found this replayable bounded counterexample.”
- “Guarded Continuation Checker is under controlled design-partner evaluation.”

Not acceptable:

- “Guarded Continuation Checker is ISO 26262/IEC 61508/IEC 62304 certified.”
- “Guarded Continuation Checker makes the firmware, RTL, device, or vehicle safe.”
- “A SAFE result proves the system can never fail.”
- “Guarded Continuation Checker is a qualified tool” without a project-specific,
  independently accepted qualification basis.

## Source snapshot

The applicability boundary above was reviewed on 2026-07-17 against primary
publisher or regulator summaries:

- [ISO 26262 series overview](https://www.iso.org/publication/PUB200262.html)
  and [ISO 26262-1:2018 scope](https://www.iso.org/standard/68383.html);
- [IEC 61508-1:2010 publication record](https://webstore.iec.ch/en/publication/5515);
- [IEC 62304:2006+A1:2015 publication record](https://webstore.iec.ch/en/publication/6792);
- [FDA infusion-pump total product lifecycle guidance](https://www.fda.gov/media/78369/download);
- [IEC 81001-5-1:2021 publication record](https://webstore.iec.ch/en/publication/63293).

Publisher summaries are informative, not substitutes for normative standards.
IEC 61508 edition 3 and IEC 62304 edition 2 were under development at this
snapshot. Reconfirm the applicable edition and regulatory status before each
production programme or assurance-plan revision.
