#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 RESULTS.csv" >&2
  exit 2
fi

results=$1
[[ -f "$results" && ! -L "$results" ]] || {
  echo "results must be an ordinary file" >&2
  exit 2
}

expected_header='schema_version,trial,channels,logical_properties,representative_properties,logical_safe,logical_unsafe,orbit_artifact_bytes,direct_artifact_bytes,orbit_evidence_bytes,direct_evidence_bytes,orbit_produce_micros,direct_produce_micros,orbit_verify_micros,direct_verify_micros,orbit_sha256,direct_sha256,answers_equal,deterministic,process_scope,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 16 ]]

awk -F, '
  NR == 1 { next }
  NF != 21 || $1 != 1 || $2 < 1 || $2 > 5 ||
  ($3 != 2 && $3 != 4 && $3 != 6) || $4 != $3 * 5 || $5 != 5 ||
  $6 + $7 != $4 || $8 >= $9 || $10 >= $11 ||
  $16 !~ /^[0-9a-f]{64}$/ || $17 !~ /^[0-9a-f]{64}$/ ||
  $18 != "true" || $19 != "true" ||
  $20 != "single-process-release" || $21 != "accepted" { exit 1 }
  {
    key = $3 ":" $2
    if (seen[key]++) exit 1
    trials[$3]++
    if (!(($3 in orbit_hash) || (orbit_hash[$3] = $16))) exit 1
    if (!(($3 in direct_hash) || (direct_hash[$3] = $17))) exit 1
    if (orbit_hash[$3] != $16 || direct_hash[$3] != $17) exit 1
    orbit[$3] = $8
    direct[$3] = $9
    orbit_evidence[$3] = $10
    direct_evidence[$3] = $11
    safe[$3] = $6
    unsafe[$3] = $7
  }
  END {
    if (trials[2] != 5 || trials[4] != 5 || trials[6] != 5) exit 1
    if (orbit[2] != 3222 || direct[2] != 5698 ||
        orbit_evidence[2] != 2706 || direct_evidence[2] != 5412 ||
        safe[2] != 8 || unsafe[2] != 2) exit 1
    if (orbit[4] != 3834 || direct[4] != 13200 ||
        orbit_evidence[4] != 3186 || direct_evidence[4] != 12754 ||
        safe[4] != 16 || unsafe[4] != 4) exit 1
    if (orbit[6] != 4446 || direct[6] != 22622 ||
        orbit_evidence[6] != 3666 || direct_evidence[6] != 22016 ||
        safe[6] != 24 || unsafe[6] != 6) exit 1
  }
' "$results"

echo "btor2_family_orbit_probe_results_v1=PASS rows=15 result=$results"
