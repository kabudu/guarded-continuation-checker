# Non-confidential evaluation outcome

The design partner and independent aggregate reviewer complete this after the
private evaluation. This is the only evaluation artifact intended for the
individual maintainer. Do not include RTL, property text, traces, credentials,
private paths, identity mappings, contractual terms, or intermediate findings.

## Identification

- Opaque evaluation ID:
- Evaluated annotated release tag:
- Evaluated commit:
- Assessment date:
- Partner identity disclosure: `PUBLIC`, `REDACTED`, or `PRIVATE`
- Independent reviewer identity disclosure: `PUBLIC`, `REDACTED`, or `PRIVATE`
- Public report references, if authorised:

## Outcome

- Overall suitability: `SUITABLE`, `SUITABLE_WITH_LIMITATIONS`,
  `NOT_SUITABLE`, or `INCONCLUSIVE`
- Suitable product/control domains:
- Unsuitable or untested domains:
- Suitable RTL/toolchain characteristics:
- Unsupported or problematic characteristics:
- Production-gate result: `PASS` or `FAIL`
- Recommendation: `PROCEED`, `REMEDIATE_AND_REPEAT`, or `DO_NOT_PROCEED`

“Suitable” means suitable only for the bounded verification-support role and
conditions stated here. It does not mean certified, qualified, conformant, safe
for a whole product, or proven beyond the evaluated horizons.

## Non-confidential aggregate evidence

- Organisations / projects / domains / workers:
- Security-review / technical-review / partner-pilot rows:
- Expected SAFE / UNSAFE / FAILURE counts:
- CQ SAFE / UNSAFE / FAILURE counts:
- Oracle disagreements found / reconciled / unresolved:
- Timeouts / resource limits / crashes / unsupported inputs:
- UNSAFE witnesses replayed and triaged:
- Repeated results and validated bundles:
- Open critical / high / other findings:
- Operator exercises completed:
- Register syntax-check result:
- Production-gate checker command version and result:

## Material limitations

List every limitation needed to interpret suitability, including untested
constructs, toolchains, domains, horizons, resource ranges, threat assumptions,
redactions, residual findings, or evidence that could not be independently
verified. Do not omit an adverse result because details are confidential; state
its non-confidential class and effect on suitability.

## Independent conclusion

- Independent reviewer conclusion:
- Basis for independence and competence:
- Confirmation that all attempted cases remain in private denominators:
- Confirmation that no unresolved disagreement was hidden by redaction:
- Confirmation that this report contains no partner-confidential material:
- Attributable reviewer approval and date:
- Partner approval for this exact disclosure and date:

The maintainer may report the outcome exactly as approved, link authorised
public references, or leave production gates open. The maintainer does not infer
a PASS from missing, private, ambiguous, or inconclusive evidence.
