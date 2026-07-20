#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 GCC_LINUX_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv" >&2
  exit 2
}
gcc_output=$(cd "$1" && pwd)
certifaiger_output=$(cd "$2" && pwd)
output=$3
trials=${TRIALS:-3}
[[ $trials =~ ^[1-9][0-9]*$ && $trials -le 10 ]] || { echo "TRIALS must be in 1..=10" >&2; exit 2; }
[[ ! -e "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-proof-runlim.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
binary="$gcc_output/guarded-continuation-checker"
[[ -x "$binary" ]] || { echo "missing GCC Linux binary" >&2; exit 2; }
tool_bytes=$(wc -c < "$binary" | tr -d ' ')

metric() {
  local name=$1 file=$2
  awk -v field="$name:" '$2 == field {print $3}' "$file"
}

printf '%s\n' 'schema_version,trial,operation,wall_seconds,peak_space_mb,evidence_bytes,tool_bytes,answers_agree,deterministic,status' > "$output"
reference=
for trial in $(seq 1 "$trials"); do
  artifact="$scratch/batch-$trial.proof"
  for operation in create verify; do
    args=(verify-controller-proof-mtbdd-plant-batch)
    [[ $operation == create ]] && args=(certify-controller-proof-mtbdd-plant-batch)
    docker run --rm --network none \
      -v "$gcc_output:/gcc:ro" -v "$certifaiger_output/bin:/cert:ro" \
      -v "$repo_root:/repo:ro" -v "$scratch:/out" \
      rust@sha256:8fa55b2f3ddf97471ab6a767bfa3f37e6bad0986ba823e75fea57e2a2a5c3073 \
      /cert/runlim -p -r 300 --sample-rate=1000 \
      -o "/out/$operation-$trial.runlim" \
      /gcc/guarded-continuation-checker "${args[@]}" \
      /repo/corpus/rtl/wmcontroller/physical-plant-batch-v1.txt \
      "/out/batch-$trial.proof" > "$scratch/$operation-$trial.log"
    grep -q 'members=6 safe=2 unsafe=4' "$scratch/$operation-$trial.log"
  done
  hash=$(sha256sum "$artifact" | cut -d' ' -f1)
  deterministic=true
  if [[ -z "$reference" ]]; then reference=$hash; elif [[ $hash != "$reference" ]]; then deterministic=false; fi
  evidence_bytes=$(wc -c < "$artifact" | tr -d ' ')
  for operation in create verify; do
    printf '1,%s,%s,%s,%s,%s,%s,true,%s,ok\n' \
      "$trial" "$operation" \
      "$(metric real "$scratch/$operation-$trial.runlim")" \
      "$(metric space "$scratch/$operation-$trial.runlim")" \
      "$evidence_bytes" "$tool_bytes" "$deterministic" >> "$output"
  done
done
awk -F, 'NR > 1 && $9 != "true" {exit 1}' "$output"
echo "GCC proof runlim benchmark status=MEASURED trials=$trials output=$output"
