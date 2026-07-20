#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv" >&2
  exit 2
}
ric3_output=$(cd "$1" && pwd)
certifaiger_output=$(cd "$2" && pwd)
output=$3
manifest=${output%.csv}.manifest-v1.txt
trials=${TRIALS:-3}
[[ $trials =~ ^[1-9][0-9]*$ && $trials -le 10 ]] || { echo "TRIALS must be in 1..=10" >&2; exit 2; }
[[ ! -e "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
[[ ! -e "$manifest" ]] || { echo "refusing to overwrite $manifest" >&2; exit 2; }
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-certified-evidence.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
model_bytes=$(wc -c "$repo_root"/corpus/rtl/wmcontroller/certified-baseline-v1/property-*.aag | awk 'END {print $1}')
producer_tool_bytes=$(wc -c < "$ric3_output/ric3" | tr -d ' ')
consumer_tool_bytes=$(du -sk "$certifaiger_output/bin" | awk '{print $1 * 1024}')

metric() {
  local name=$1 file=$2
  awk -v field="$name:" '$2 == field {print $3}' "$file"
}

printf '%s\n' 'schema_version,trial,operation,wall_seconds,peak_space_mb,model_bytes,evidence_bytes,producer_tool_bytes,consumer_tool_bytes,answers_agree,deterministic,status' > "$output"
reference=
for trial in $(seq 1 "$trials"); do
  trial_dir="$scratch/trial-$trial"
  mkdir "$trial_dir"
  docker run --rm --network none \
    -v "$ric3_output:/tools:ro" -v "$certifaiger_output/bin:/cert:ro" \
    -v "$repo_root:/repo:ro" -v "$trial_dir:/out" \
    "$ric3_image" \
    /cert/runlim -p -r 300 --sample-rate=1000 -o /out/producer.runlim \
    /repo/scripts/certified-evidence-container-v1.sh produce
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/cert:ro" -v "$repo_root:/repo:ro" \
    -v "$trial_dir:/out" "$certifaiger_image" \
    /cert/runlim -p -r 300 --sample-rate=1000 -o /out/consumer.runlim \
    /repo/scripts/certified-evidence-container-v1.sh consume
  hashes=$(cd "$trial_dir" && sha256sum property-*.witness.aag)
  evidence_bytes=$(wc -c "$trial_dir"/property-*.witness.aag | awk 'END {print $1}')
  deterministic=true
  if [[ -z "$reference" ]]; then
    reference=$hashes
    {
      printf 'schema_version=1\n'
      printf 'qualification_lock_sha256=%s\n' "$(sha256sum "$repo_root/tools/certifaiger-qualification-v1.lock" | cut -d' ' -f1)"
      printf 'model_manifest_sha256=%s\n' "$(sha256sum "$repo_root/corpus/rtl/wmcontroller/certified-baseline-v1/manifest-v1.txt" | cut -d' ' -f1)"
      printf 'producer_binary_sha256=%s\n' "$(sha256sum "$ric3_output/ric3" | cut -d' ' -f1)"
      printf 'consumer_tree_sha256=%s\n' "$(cd "$certifaiger_output/bin" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1)"
      printf 'evidence_bytes=%s\n' "$evidence_bytes"
      while read -r digest file; do
        property=${file#property-}
        property=${property%.witness.aag}
        printf 'property_%s_witness_sha256=%s\n' "$property" "$digest"
      done <<< "$hashes"
    } > "$manifest"
  elif [[ "$hashes" != "$reference" ]]; then
    deterministic=false
  fi
  for operation in producer consumer; do
    printf '1,%s,%s,%s,%s,%s,%s,%s,%s,true,%s,ok\n' \
      "$trial" "$operation" \
      "$(metric real "$trial_dir/$operation.runlim")" \
      "$(metric space "$trial_dir/$operation.runlim")" \
      "$model_bytes" "$evidence_bytes" "$producer_tool_bytes" \
      "$consumer_tool_bytes" "$deterministic" >> "$output"
  done
done
awk -F, 'NR > 1 && $11 != "true" {exit 1}' "$output"
echo "certified evidence benchmark status=MEASURED trials=$trials output=$output"
echo "certified evidence manifest status=RETAINED output=$manifest"
