#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv" >&2
  exit 2
}
ric3_output=$(cd "$1" && pwd)
certifaiger_output=$(cd "$2" && pwd)
output=$3
[[ ! -e "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-certified-hostile.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
base="$scratch/base"
mkdir "$base"

produce() {
  local dir=$1
  docker run --rm --network none \
    -v "$ric3_output:/tools:ro" -v "$certifaiger_output/bin:/cert:ro" \
    -v "$repo_root:/repo:ro" -v "$dir:/out" "$ric3_image" \
    /repo/scripts/certified-evidence-container-v1.sh produce >/dev/null 2>&1
}
consume() {
  local dir=$1
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/cert:ro" -v "$repo_root:/repo:ro" \
    -v "$dir:/out" "$certifaiger_image" \
    /repo/scripts/certified-evidence-container-v1.sh consume >/dev/null 2>&1
}
record_rejection() {
  local case_name=$1 case_dir=$2
  if consume "$case_dir"; then
    printf '1,%s,false,accepted-hostile-input\n' "$case_name" >> "$output"
    return 1
  fi
  printf '1,%s,true,rejected\n' "$case_name" >> "$output"
}

produce "$base"
consume "$base"
printf '%s\n' 'schema_version,case,rejected,status' > "$output"
printf '1,baseline,false,accepted-valid-evidence\n' >> "$output"

cp -R "$base" "$scratch/mutation"
printf '\001' | dd of="$scratch/mutation/property-15.witness.aag" bs=1 seek=20 conv=notrunc status=none
record_rejection mutation "$scratch/mutation"

cp -R "$base" "$scratch/truncation"
head -c 20 "$scratch/truncation/property-15.witness.aag" > "$scratch/truncated"
mv "$scratch/truncated" "$scratch/truncation/property-15.witness.aag"
record_rejection truncation "$scratch/truncation"

cp -R "$base" "$scratch/substitution"
cp "$base/property-12.witness.aag" "$scratch/substitution/property-11.witness.aag"
record_rejection property-substitution "$scratch/substitution"

cp -R "$base" "$scratch/reorder"
cp "$base/property-11.witness.aag" "$scratch/swap"
cp "$base/property-12.witness.aag" "$scratch/reorder/property-11.witness.aag"
cp "$scratch/swap" "$scratch/reorder/property-12.witness.aag"
record_rejection member-reordering "$scratch/reorder"

cp -R "$base" "$scratch/stale"
cp "$base/property-11.witness.aag" "$scratch/stale/property-16.witness.aag"
record_rejection stale-evidence "$scratch/stale"

if produce "$base"; then
  printf '1,output-collision,false,overwritten\n' >> "$output"
  exit 1
fi
printf '1,output-collision,true,rejected\n' >> "$output"

cp "$repo_root/corpus/rtl/wmcontroller/certified-baseline-v1/property-11.aag" "$scratch/drifted.aag"
printf '\001' | dd of="$scratch/drifted.aag" bs=1 seek=40 conv=notrunc status=none
if docker run --rm --network none \
  -v "$certifaiger_output/bin:/cert:ro" -v "$scratch:/hostile:ro" \
  "$certifaiger_image" \
  /cert/aigsim -c -m /hostile/drifted.aag /hostile/base/property-11.witness.aag \
  >/dev/null 2>&1; then
  printf '1,source-drift,false,accepted-hostile-input\n' >> "$output"
  exit 1
fi
printf '1,source-drift,true,rejected\n' >> "$output"

echo "certified evidence hostile controls status=PASS output=$output"
