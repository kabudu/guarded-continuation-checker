#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
results="$repo_root/results"

arm_external="$results/certified-evidence-equivalent-arm64-v1.csv"
amd_external="$results/certified-evidence-equivalent-amd64-v1.csv"
arm_manifest="$results/certified-evidence-equivalent-arm64-v1.manifest-v1.txt"
amd_manifest="$results/certified-evidence-equivalent-amd64-v1.manifest-v1.txt"
arm_hostile="$results/certified-evidence-hostile-arm64-v1.csv"
amd_hostile="$results/certified-evidence-hostile-amd64-v1.csv"
amd_gcc="$results/gcc-proof-equivalent-amd64-v1.csv"
amd_gcc_manifest="$results/gcc-proof-equivalent-amd64-v1.manifest-v1.txt"
cross_gcc="$results/gcc-proof-cross-platform-v1.txt"
opentitan_composed="$results/opentitan-dual-timer-composed-witness-v1.csv"
opentitan_manifest="$results/opentitan-dual-timer-composed-witness-v1.manifest.txt"
opentitan_amd_composed="$results/opentitan-dual-timer-composed-witness-amd64-v1.csv"
opentitan_amd_manifest="$results/opentitan-dual-timer-composed-witness-amd64-v1.manifest.txt"
opentitan_amd_resources="$results/opentitan-dual-timer-resources-amd64-v1.csv"
opentitan_amd_resource_manifest="$results/opentitan-dual-timer-resources-amd64-v1.manifest.txt"
opentitan_amd_provenance="$results/opentitan-dual-timer-hosted-amd64-v1.provenance.txt"
caliptra_composed="$results/caliptra-wdt-composed-witness-v1.csv"
caliptra_manifest="$results/caliptra-wdt-composed-witness-v1.manifest.txt"

for file in "$arm_external" "$amd_external" "$arm_manifest" "$amd_manifest" \
  "$arm_hostile" "$amd_hostile" "$amd_gcc" "$amd_gcc_manifest" "$cross_gcc" \
  "$opentitan_composed" "$opentitan_manifest" \
  "$opentitan_amd_composed" "$opentitan_amd_manifest" \
  "$opentitan_amd_resources" "$opentitan_amd_resource_manifest" \
  "$opentitan_amd_provenance" "$caliptra_composed" "$caliptra_manifest"; do
  [[ -f "$file" && ! -L "$file" ]] || { echo "missing retained evidence: $file" >&2; exit 1; }
done

check_external_csv() {
  awk -F, '
    NR == 1 { next }
    NF != 12 || $10 != "true" || $11 != "true" || $12 != "ok" { exit 1 }
    END { if (NR != 7) exit 1 }
  ' "$1"
}
check_gcc_csv() {
  awk -F, '
    NR == 1 { next }
    NF != 10 || $8 != "true" || $9 != "true" || $10 != "ok" { exit 1 }
    END { if (NR != 7) exit 1 }
  ' "$1"
}
check_hostile_csv() {
  awk -F, '
    NR == 2 && ($2 != "baseline" || $3 != "false" || $4 != "accepted-valid-evidence") { exit 1 }
    NR > 2 && ($3 != "true" || $4 != "rejected") { exit 1 }
    END { if (NR != 9) exit 1 }
  ' "$1"
}

check_external_csv "$arm_external"
check_external_csv "$amd_external"
check_gcc_csv "$amd_gcc"
check_hostile_csv "$arm_hostile"
check_hostile_csv "$amd_hostile"
cmp -s "$arm_hostile" "$amd_hostile"

check_opentitan_composed() {
  awk -F, '
  NR == 1 { next }
  NF != 12 || $10 != "true" || $11 != "true" || $12 != "validated" { exit 1 }
  $4 == "SAFE" && ($5 != "UNSAT" || $6 != "none") { exit 1 }
  $4 == "UNSAFE" && ($5 != "SAT" || $6 !~ /^(5|7|9)$/) { exit 1 }
  $4 == "SAFE" && $7 != "ic3" { exit 1 }
  $4 == "UNSAFE" && $7 != "bmc" { exit 1 }
  { key = $2 ":" $3; if (!(key in seen)) count++; seen[key]++; answers[key] = $4 ":" $6 }
  END {
    if (NR != 13 || count != 12) exit 1
    if (answers["4:wake"] != "SAFE:none" || answers["4:bark"] != "SAFE:none" || answers["4:bite"] != "SAFE:none") exit 1
    if (answers["5:wake"] != "SAFE:none" || answers["5:bark"] != "UNSAFE:5" || answers["5:bite"] != "SAFE:none") exit 1
    if (answers["7:wake"] != "UNSAFE:7" || answers["7:bark"] != "UNSAFE:5" || answers["7:bite"] != "SAFE:none") exit 1
    if (answers["9:wake"] != "UNSAFE:7" || answers["9:bark"] != "UNSAFE:5" || answers["9:bite"] != "UNSAFE:9") exit 1
  }
' "$1"
}
check_opentitan_composed "$opentitan_composed"
check_opentitan_composed "$opentitan_amd_composed"
[[ $(sed -n 's/^answer_count=//p' "$opentitan_manifest") == 12 ]]
[[ $(sed -n 's/^safe_certificate_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^unsafe_trace_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^safe_producer_engine=//p' "$opentitan_manifest") == ric3-ic3 ]]
[[ $(sed -n 's/^unsafe_producer_engine=//p' "$opentitan_manifest") == ric3-bmc ]]
[[ $(sed -n 's/^unsafe_trace_contract=//p' "$opentitan_manifest") == earliest-bad-frame ]]
[[ $(sed -n 's/^model_serialization_profile=//p' "$opentitan_manifest") == \
  canonical-yosys-revision-v1 ]]
[[ $(sed -n 's/^composed_safe_set_count=//p' "$opentitan_manifest") == 2 ]]
[[ $(sed -n 's/^hostile_control_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^status=//p' "$opentitan_manifest") == validated ]]
[[ $(sed -n 's/^status=//p' "$opentitan_amd_manifest") == validated ]]
[[ $(sed -n 's/^model_serialization_profile=//p' \
  "$opentitan_amd_manifest") == canonical-yosys-revision-v1 ]]

cmp "$opentitan_composed" "$opentitan_amd_composed"
diff -u \
  <(grep -Ev '^(ric3_binary_sha256|certifaiger_tree_sha256)=' \
    "$opentitan_manifest") \
  <(grep -Ev '^(ric3_binary_sha256|certifaiger_tree_sha256)=' \
    "$opentitan_amd_manifest")

awk -F, '
  NR == 1 { next }
  NF != 13 || $6 <= 0 || $7 <= 0 || $8 <= 0 || $9 <= 0 ||
    $10 <= 0 || $11 <= 0 || $12 != "true" || $13 != "measured" { exit 1 }
  $2 !~ /^[1-3]$/ || $3 !~ /^(4|5)$/ || $4 !~ /^(gcc|external)$/ ||
    $5 !~ /^(producer|consumer)$/ { exit 1 }
  {
    key = $2 ":" $3 ":" $4 ":" $5
    if (!(key in seen)) count++
    seen[key]++
  }
  END {
    if (NR != 25 || count != 24) exit 1
    for (key in seen) if (seen[key] != 1) exit 1
  }
' "$opentitan_amd_resources"
[[ $(sed -n 's/^trials=//p' "$opentitan_amd_resource_manifest") == 3 ]]
[[ $(sed -n 's/^external_producer_policy=//p' \
  "$opentitan_amd_resource_manifest") == static-ic3-safe-bmc-earliest-unsafe-race ]]
[[ $(sed -n 's/^model_serialization_profile=//p' \
  "$opentitan_amd_resource_manifest") == canonical-yosys-revision-v1 ]]
[[ $(sed -n 's/^status=//p' "$opentitan_amd_resource_manifest") == measured ]]

sha256_portable() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
for binding in \
  "composed_csv_sha256:$opentitan_amd_composed" \
  "composed_manifest_sha256:$opentitan_amd_manifest" \
  "resources_csv_sha256:$opentitan_amd_resources" \
  "resources_manifest_sha256:$opentitan_amd_resource_manifest"; do
  field=${binding%%:*}
  file=${binding#*:}
  [[ $(sed -n "s/^$field=//p" "$opentitan_amd_provenance") == \
    $(sha256_portable "$file") ]]
done
[[ $(sed -n 's/^workflow_head_sha=//p' "$opentitan_amd_provenance") == \
  9656749ce892bcbaf66d7a38ed570f59cab1e2a6 ]]
[[ $(sed -n 's/^status=//p' "$opentitan_amd_provenance") == retained ]]

awk -F, '
  NR == 1 { next }
  NF != 12 || $10 != "true" || $11 != "true" || $12 != "validated" { exit 1 }
  $4 == "SAFE" && ($5 != "UNSAT" || $6 != "none" || $7 != "ic3") { exit 1 }
  $4 == "UNSAFE" && ($5 != "SAT" || $6 !~ /^(3|5)$/ || $7 != "bmc") { exit 1 }
  { key = $2 ":" $3; if (!(key in seen)) count++; seen[key]++; answers[key] = $4 ":" $6 }
  END {
    if (NR != 10 || count != 9) exit 1
    if (answers["2:t1"] != "SAFE:none" || answers["2:t2"] != "SAFE:none" || answers["2:fatal"] != "SAFE:none") exit 1
    if (answers["3:t1"] != "UNSAFE:3" || answers["3:t2"] != "SAFE:none" || answers["3:fatal"] != "SAFE:none") exit 1
    if (answers["5:t1"] != "UNSAFE:3" || answers["5:t2"] != "UNSAFE:5" || answers["5:fatal"] != "UNSAFE:5") exit 1
  }
' "$caliptra_composed"
[[ $(sed -n 's/^model_count=//p' "$caliptra_manifest") == 11 ]]
[[ $(sed -n 's/^answer_count=//p' "$caliptra_manifest") == 9 ]]
[[ $(sed -n 's/^safe_certificate_count=//p' "$caliptra_manifest") == 5 ]]
[[ $(sed -n 's/^unsafe_trace_count=//p' "$caliptra_manifest") == 4 ]]
[[ $(sed -n 's/^model_serialization_profile=//p' "$caliptra_manifest") == \
  canonical-yosys-revision-v1 ]]
[[ $(sed -n 's/^composed_safe_set_count=//p' "$caliptra_manifest") == 2 ]]
[[ $(sed -n 's/^hostile_control_count=//p' "$caliptra_manifest") == 6 ]]
[[ $(sed -n 's/^status=//p' "$caliptra_manifest") == validated ]]

shared_pattern='^(qualification_lock_sha256|model_manifest_sha256|evidence_bytes|property_.*)='
diff -u \
  <(grep -E "$shared_pattern" "$arm_manifest") \
  <(grep -E "$shared_pattern" "$amd_manifest")

amd_proof=$(sed -n 's/^batch_proof_sha256=//p' "$amd_gcc_manifest")
cross_proof=$(sed -n 's/^batch_proof_sha256=//p' "$cross_gcc")
[[ -n "$amd_proof" && "$amd_proof" == "$cross_proof" ]]
[[ $(sed -n 's/^cross_platform_byte_identity=//p' "$cross_gcc") == true ]]
[[ $(sed -n 's/^evidence_bytes=//p' "$amd_gcc_manifest") == 251221 ]]

echo "retained-certified-evidence-check=PASS"
