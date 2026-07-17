#!/usr/bin/env bash
set -euo pipefail

readonly HEADER='record_id,record_type,custodian_id,organization_id,project_id,domain_id,worker_id,target_tag,target_commit,environment_id,input_digest,requirement_digest,expected_result_source,expected_result,cq_result,oracle_result,exit_class,bundle_digest,isolation_report_digest,runtime_ms,peak_memory_bytes,witness_replayed,partner_triaged,repeat_result,repeat_bundle_valid,disposition,review_status,report_reference'
readonly MAX_BYTES=8388608
readonly MAX_ROWS=10000

usage() {
  echo "usage: $0 REGISTER.csv | $0 --production-gate REGISTER.csv ATTESTATION.conf SOURCE_REPOSITORY" >&2
  exit 2
}

mode=validate
if [[ $# -eq 1 ]]; then
  register=$1
elif [[ $# -eq 4 && $1 == --production-gate ]]; then
  mode=production-gate
  register=$2
  attestation=$3
  source_repository=$4
else
  usage
fi

[[ -f "$register" && ! -L "$register" ]] || {
  echo "evidence register must be a regular non-symlink file: $register" >&2
  exit 2
}
register_bytes=$(wc -c < "$register")
(( register_bytes <= MAX_BYTES )) || {
  echo "evidence register exceeds $MAX_BYTES bytes" >&2
  exit 2
}
[[ $(head -n 1 "$register") == "$HEADER" ]] || {
  echo "evidence register header does not match protocol v1" >&2
  exit 2
}
if LC_ALL=C grep -q $'[\r"]' "$register"; then
  echo "evidence register must use the unquoted LF-only protocol profile" >&2
  exit 2
fi

awk -F, -v max_rows="$MAX_ROWS" '
  function fail(message) {
    print "evidence register line " NR ": " message > "/dev/stderr"
    exit 2
  }
  function identifier(value) { return value ~ /^[A-Za-z0-9._-]+$/ }
  function digest(value) {
    return length(value) == 71 && value ~ /^sha256:[0-9a-f]+$/
  }
  function result(value) { return value == "SAFE" || value == "UNSAFE" || value == "FAILURE" }
  function boolean_na(value) { return value == "yes" || value == "no" || value == "na" }
  NR == 1 { next }
  {
    if (NR - 1 > max_rows) fail("row limit exceeded")
    if (length($0) > 16384) fail("line exceeds 16384 bytes")
    if (NF != 28) fail("expected 28 fields, found " NF)
    for (i = 1; i <= NF; i++) {
      if (substr($i, 1, 1) ~ /^[=+@-]$/) fail("spreadsheet formula prefix in field " i)
    }
    if (!identifier($1) || seen_record[$1]++) fail("invalid or duplicate record_id")
    if ($2 != "security-review" && $2 != "technical-review" && $2 != "partner-pilot") fail("invalid record_type")
    for (i = 3; i <= 10; i++) {
      if (i != 9 && !identifier($i)) fail("invalid identifier in field " i)
    }
    if (length($9) != 40 || $9 !~ /^[0-9a-f]+$/) fail("target_commit must be a full lowercase SHA-1 object ID")
    if (!digest($11) || !digest($12)) fail("invalid input or requirement digest")
    if ($13 == "" || $13 !~ /^[A-Za-z0-9._:\/?#&%=-]+$/) fail("invalid expected_result_source")
    if (!result($14) || !result($15) || !result($16)) fail("invalid result enum")
    if ($17 != "0" && $17 != "1" && $17 != "2") fail("invalid exit_class")
    if ($14 != "FAILURE" && (!digest($18) || !digest($19))) fail("SAFE/UNSAFE rows require bundle and isolation digests")
    if ($14 == "FAILURE" && (($18 != "" && !digest($18)) || ($19 != "" && !digest($19)))) fail("invalid optional failure digest")
    if ($20 !~ /^[0-9]+$/ || $21 !~ /^[0-9]+$/) fail("runtime and memory must be decimal integers")
    if (!boolean_na($22) || !boolean_na($23) || !boolean_na($24) || !boolean_na($25)) fail("invalid yes/no/na field")
    if ($26 != "accepted" && $26 != "reconciled" && $26 != "open") fail("invalid disposition")
    if ($27 != "pending" && $27 != "reviewed" && $27 != "rejected") fail("invalid review_status")
    if ($28 == "" || $28 !~ /^[A-Za-z0-9._:\/?#&%=-]+$/) fail("invalid report_reference")
  }
' "$register"

rows=$(awk 'END { print NR - 1 }' "$register")
if [[ $mode == validate ]]; then
  echo "external-evidence-register status=VALID rows=$rows"
  exit 0
fi

[[ -f "$attestation" && ! -L "$attestation" ]] || {
  echo "attestation must be a regular non-symlink file: $attestation" >&2
  exit 2
}
attestation_bytes=$(wc -c < "$attestation")
(( attestation_bytes <= 65536 )) || { echo "attestation exceeds 65536 bytes" >&2; exit 2; }
if LC_ALL=C grep -q $'[\r,\"]' "$attestation"; then
  echo "attestation contains a prohibited CR, comma, or quote" >&2
  exit 2
fi

awk -F, '
  function fail(message) {
    print "external production gate: " message > "/dev/stderr"
    exit 1
  }
  function digest(value) { return length(value) == 71 && value ~ /^sha256:[0-9a-f]+$/ }
  function identifier(value) { return value ~ /^[A-Za-z0-9._-]+$/ }
  function reference(value) { return value ~ /^[A-Za-z0-9._:\/?#&%=-]+$/ }
  function load_attestation(line, position, key, value) {
    position = index(line, "=")
    if (!position) fail("invalid attestation line")
    key = substr(line, 1, position - 1)
    value = substr(line, position + 1)
    if (key !~ /^[a-z_]+$/ || value == "" || seen_key[key]++) fail("invalid or duplicate attestation key")
    if (substr(value, 1, 1) ~ /^[=+@-]$/) fail("formula prefix in attestation")
    attest[key] = value
  }
  FNR == NR {
    load_attestation($0)
    next
  }
  FNR == 1 { next }
  {
    rows++
    if ($8 != attest["target_tag"] || $9 != attest["target_commit"]) fail("mixed or incorrect target release")
    if ($14 != $15 || $14 != $16) fail("unresolved expected/CQ/oracle disagreement")
    if (($15 == "SAFE" && $17 != "0") || ($15 == "UNSAFE" && $17 != "1") || ($15 == "FAILURE" && $17 != "2")) fail("result/exit mismatch")
    if ($20 + 0 <= 0 || $21 + 0 <= 0) fail("runtime and peak memory must be positive")
    if ($24 != "yes" || $25 != "yes") fail("repeat result and bundle validation must pass")
    if ($26 == "open" || $27 != "reviewed") fail("row is unresolved or not reviewed")
    if ($2 == "security-review") security++
    if ($2 == "technical-review") technical++
    if ($2 == "partner-pilot") {
      partner++
      organisations[$4] = 1
      projects[$5] = 1
      domains[$6] = 1
      workers[$7] = 1
      partner_results[$14]++
      tuple = $11 SUBSEP $12
      if (seen_partner_tuple[tuple]++) fail("duplicate partner input/requirement configuration")
      if ($14 == "UNSAFE" && ($22 != "yes" || $23 != "yes")) fail("UNSAFE witness was not replayed and partner-triaged")
    }
  }
  END {
    required["protocol_version"] = "1"
    required["security_assessment_status"] = "PASS"
    required["technical_review_status"] = "PASS"
    required["operator_exercises_status"] = "PASS"
    required["data_handling_status"] = "PASS"
    required["independent_aggregate_status"] = "PASS"
    required["critical_findings_open"] = "0"
    required["high_findings_open"] = "0"
    for (key in required) if (attest[key] != required[key]) fail("attestation requirement failed: " key)
    needed["target_tag"] = 1
    needed["target_commit"] = 1
    needed["security_assessment_report"] = 1
    needed["technical_review_report"] = 1
    needed["independent_reviewer_id"] = 1
    needed["independent_aggregate_report"] = 1
    needed["assessment_date"] = 1
    for (key in needed) if (!(key in attest) || attest[key] == "") fail("missing attestation key: " key)
    if (attest["target_tag"] !~ /^v[0-9]+\.[0-9]+\.[0-9]+$/) fail("invalid attestation target_tag")
    if (length(attest["target_commit"]) != 40 || attest["target_commit"] !~ /^[0-9a-f]+$/) fail("invalid attestation target_commit")
    if (attest["assessment_date"] !~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/) fail("invalid assessment_date")
    month = substr(attest["assessment_date"], 6, 2) + 0
    day = substr(attest["assessment_date"], 9, 2) + 0
    if (month < 1 || month > 12 || day < 1 || day > 31) fail("invalid assessment_date range")
    if (!identifier(attest["independent_reviewer_id"])) fail("invalid independent_reviewer_id")
    if (!reference(attest["security_assessment_report"]) || !reference(attest["technical_review_report"]) || !reference(attest["independent_aggregate_report"])) fail("invalid attestation report reference")
    for (key in seen_key) key_count++
    if (key_count != 15) fail("attestation must contain exactly 15 keys")
    if (security < 7) fail("fewer than 7 security-review cases")
    if (technical < 3) fail("fewer than 3 technical-review cases")
    if (partner < 30) fail("fewer than 30 partner-pilot configurations")
    for (key in organisations) organisation_count++
    for (key in projects) project_count++
    for (key in domains) domain_count++
    for (key in workers) worker_count++
    if (organisation_count < 2 || project_count < 3 || domain_count < 2 || worker_count < 2) fail("partner cohort diversity is insufficient")
    if (partner_results["SAFE"] < 5 || partner_results["UNSAFE"] < 5 || partner_results["FAILURE"] < 5) fail("partner result-class coverage is insufficient")
  }
' "$attestation" "$register"

[[ -d "$source_repository" ]] || {
  echo "external production gate: source repository is not a directory" >&2
  exit 1
}
target_tag=$(sed -n 's/^target_tag=//p' "$attestation")
target_commit=$(sed -n 's/^target_commit=//p' "$attestation")
if [[ $(git -C "$source_repository" cat-file -t "refs/tags/$target_tag" 2>/dev/null || true) != tag ]]; then
  echo "external production gate: target must be an annotated tag in the source repository" >&2
  exit 1
fi
resolved_commit=$(git -C "$source_repository" rev-parse --verify "refs/tags/$target_tag^{}" 2>/dev/null || true)
if [[ "$resolved_commit" != "$target_commit" ]]; then
  echo "external production gate: target tag does not resolve to the attested commit" >&2
  exit 1
fi

echo "external-production-gate status=PASS rows=$rows"
