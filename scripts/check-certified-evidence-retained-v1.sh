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

for file in "$arm_external" "$amd_external" "$arm_manifest" "$amd_manifest" \
  "$arm_hostile" "$amd_hostile" "$amd_gcc" "$amd_gcc_manifest" "$cross_gcc" \
  "$opentitan_composed" "$opentitan_manifest"; do
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

awk -F, '
  NR == 1 { next }
  NF != 11 || $9 != "true" || $10 != "true" || $11 != "validated" { exit 1 }
  $4 == "SAFE" && ($5 != "UNSAT" || $6 != "none") { exit 1 }
  $4 == "UNSAFE" && ($5 != "SAT" || $6 !~ /^(5|7|9)$/) { exit 1 }
  { key = $2 ":" $3; if (!(key in seen)) count++; seen[key]++; answers[key] = $4 ":" $6 }
  END {
    if (NR != 13 || count != 12) exit 1
    if (answers["4:wake"] != "SAFE:none" || answers["4:bark"] != "SAFE:none" || answers["4:bite"] != "SAFE:none") exit 1
    if (answers["5:wake"] != "SAFE:none" || answers["5:bark"] != "UNSAFE:5" || answers["5:bite"] != "SAFE:none") exit 1
    if (answers["7:wake"] != "UNSAFE:7" || answers["7:bark"] != "UNSAFE:5" || answers["7:bite"] != "SAFE:none") exit 1
    if (answers["9:wake"] != "UNSAFE:7" || answers["9:bark"] != "UNSAFE:5" || answers["9:bite"] != "UNSAFE:9") exit 1
  }
' "$opentitan_composed"
[[ $(sed -n 's/^answer_count=//p' "$opentitan_manifest") == 12 ]]
[[ $(sed -n 's/^safe_certificate_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^unsafe_trace_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^composed_safe_set_count=//p' "$opentitan_manifest") == 2 ]]
[[ $(sed -n 's/^hostile_control_count=//p' "$opentitan_manifest") == 6 ]]
[[ $(sed -n 's/^status=//p' "$opentitan_manifest") == validated ]]

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
