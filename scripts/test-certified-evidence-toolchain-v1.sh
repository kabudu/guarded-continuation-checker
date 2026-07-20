#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT_DIR" >&2
  exit 2
}
ric3_output=$(cd "$1" && pwd)
certifaiger_output=$(cd "$2" && pwd)
output_dir=$3
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
[[ ! -e "$output_dir" ]] || { echo "refusing to overwrite $output_dir" >&2; exit 2; }
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)

safe_model=/repo/examples/products/infusion-pump/firmware/safe-controller.aag
unsafe_model=/repo/examples/products/infusion-pump/firmware/door-interlock-regression.aag

docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$repo_root:/repo:ro" -v "$output_dir:/out" \
  gcc-ric3-qualification:v1-arm64 \
  /tools/ric3 check "$safe_model" --cert /out/safe-certificate.aag --ui false ic3 \
  > "$output_dir/safe-producer.log" 2>&1
docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$repo_root:/repo:ro" \
  -v "$output_dir:/out:ro" gcc-certifaiger-qualification:v1-arm64 \
  /tools/check "$safe_model" /out/safe-certificate.aag \
  > "$output_dir/safe-consumer.log" 2>&1

docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$repo_root:/repo:ro" -v "$output_dir:/out" \
  gcc-ric3-qualification:v1-arm64 \
  /tools/ric3 check "$unsafe_model" --cert /out/unsafe-trace.aag --ui false ic3 \
  > "$output_dir/unsafe-producer.log" 2>&1
docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$repo_root:/repo:ro" \
  -v "$output_dir:/out:ro" gcc-certifaiger-qualification:v1-arm64 \
  /tools/aigsim -c -m "$unsafe_model" /out/unsafe-trace.aag \
  > "$output_dir/unsafe-consumer.log" 2>&1

sha256sum "$output_dir/safe-certificate.aag" "$output_dir/unsafe-trace.aag" \
  > "$output_dir/evidence.sha256"
printf 'safe=pass\nunsafe=pass\n' > "$output_dir/result.txt"
