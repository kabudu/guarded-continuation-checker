#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 CQ_BINARY WORK_DIR" >&2
  exit 2
fi

if [[ $(uname -s) != Linux ]]; then
  echo "production evaluation requires Linux containment" >&2
  exit 2
fi

script_dir=$(cd "$(dirname "$0")" && pwd)
repository=$(cd "$script_dir/.." && pwd)
[[ -f "$1" ]] || { echo "CQ binary is missing: $1" >&2; exit 2; }
cq_binary=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
work_dir=$2

[[ -x "$cq_binary" ]] || { echo "CQ binary is not executable: $cq_binary" >&2; exit 2; }
command -v yosys >/dev/null 2>&1 || { echo "Yosys is required" >&2; exit 2; }
[[ ! -e "$work_dir" ]] || { echo "work directory must not already exist: $work_dir" >&2; exit 2; }
mkdir -p "$work_dir"

require_line() {
  local expected=$1
  local file=$2
  grep -qx "$expected" "$file" || {
    echo "qualification field mismatch in $file: expected $expected" >&2
    exit 1
  }
}

version=$("$cq_binary" firmware-cli-version)
[[ "$version" == "firmware_cli_version=2 artifact_schema_version=4" ]] || {
  echo "unexpected CQ compatibility contract: $version" >&2
  exit 1
}

safe="$work_dir/safe"
unsafe="$work_dir/unsafe"
"$cq_binary" firmware-rtl-config-safety-gate \
  "$repository/corpus/rtl/yosys-simple/always01-safe.conf" "$safe"
"$cq_binary" firmware-artifact-validate "$safe"

set +e
"$cq_binary" firmware-rtl-config-safety-gate \
  "$repository/corpus/rtl/yosys-simple/always01-unsafe.conf" "$unsafe"
unsafe_exit=$?
set -e
[[ $unsafe_exit -eq 1 ]] || {
  echo "unsafe smoke case returned $unsafe_exit instead of 1" >&2
  exit 1
}
"$cq_binary" firmware-artifact-validate "$unsafe"

for bundle in "$safe" "$unsafe"; do
  require_line 'containment_platform=linux' "$bundle/run-manifest.txt"
  require_line 'synthesis_memory_limit_kind=address-space' "$bundle/run-manifest.txt"
  require_line 'synthesis_memory_limit_bytes=2147483648' "$bundle/run-manifest.txt"
  require_line 'synthesis_file_limit_bytes=536870912' "$bundle/run-manifest.txt"
  require_line 'evidence_digest_algorithm=sha256' "$bundle/run-manifest.txt"
done

require_line 'status=SAFE' "$safe/run-manifest.txt"
require_line 'status=UNSAFE' "$unsafe/run-manifest.txt"

printf 'production-evaluation-check=PASS\n%s\n' "$version"
yosys -V
