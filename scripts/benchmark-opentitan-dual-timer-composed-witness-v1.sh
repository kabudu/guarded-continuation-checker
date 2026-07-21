#!/usr/bin/env bash
set -euo pipefail

report_failure() {
  local status=$?
  echo "opentitan composed-witness failure status=$status line=${BASH_LINENO[0]} command=$BASH_COMMAND" >&2
  exit "$status"
}
trap report_failure ERR

if [[ $# -ne 6 ]]; then
  echo "usage: $0 GCC_BINARY YOSYS_BINARY RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv MANIFEST.txt" >&2
  exit 2
fi

gcc_binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
yosys_binary=$(cd "$(dirname "$2")" && pwd -P)/$(basename "$2")
ric3_output=$(cd "$3" && pwd -P)
certifaiger_output=$(cd "$4" && pwd -P)
output=$5
manifest=$6
repository=$(cd "$(dirname "$0")/.." && pwd -P)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}

[[ -x "$gcc_binary" && -x "$yosys_binary" && -x "$ric3_output/ric3" ]] || {
  echo "required producer binary is unavailable" >&2
  exit 2
}
[[ -x "$certifaiger_output/bin/check" && -x "$certifaiger_output/bin/aigsim" ]] || {
  echo "required independent checker binary is unavailable" >&2
  exit 2
}
for target in "$output" "$manifest"; do
  [[ ! -e "$target" && ! -L "$target" ]] || {
    echo "refusing to overwrite $target" >&2
    exit 2
  }
done

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-composed.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
models=$scratch/models
models_second=$scratch/models-second
evidence=$scratch/evidence
mkdir "$evidence"
"$repository/scripts/build-opentitan-aon-dual-timer-aiger.sh" \
  "$yosys_binary" "$models" >/dev/null
"$repository/scripts/build-opentitan-aon-dual-timer-aiger.sh" \
  "$yosys_binary" "$models_second" >/dev/null
diff -ru "$models" "$models_second" >/dev/null
echo "opentitan composed-witness phase=models-deterministic"
models=$(cd "$models" && pwd -P)
evidence=$(cd "$evidence" && pwd -P)

sha256sum_portable() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

tree_sha256_portable() {
  directory=$1
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$directory" && find . -type f -print0 | sort -z | \
      xargs -0 sha256sum | sha256sum | awk '{print $1}')
  else
    (cd "$directory" && find . -type f -print0 | sort -z | \
      xargs -0 shasum -a 256 | shasum -a 256 | awk '{print $1}')
  fi
}

expected_answer() {
  case "$1:$2" in
    4:*) echo SAFE ;;
    5:wake|5:bite|7:bite) echo SAFE ;;
    5:bark|7:wake|7:bark|9:*) echo UNSAFE ;;
    *) echo "invalid case $1:$2" >&2; exit 2 ;;
  esac
}

expected_frame() {
  case "$1" in
    wake) echo 7 ;;
    bark) echo 5 ;;
    bite) echo 9 ;;
    *) exit 2 ;;
  esac
}

produce() {
  local model=$1 destination=$2 log=$3
  docker run --rm --network none \
    -v "$ric3_output:/tools:ro" -v "$models:/models:ro" \
    -v "$evidence:/out" "$ric3_image" \
    /tools/ric3 check "/models/$model" --cert "/out/$destination" \
    --ui false ic3 >"$evidence/$log" 2>&1
}

verify_safe() {
  local model=$1 certificate=$2 log=$3
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
    -v "$evidence:/out:ro" "$certifaiger_image" \
    /tools/check "/models/$model" "/out/$certificate" \
    >"$evidence/$log" 2>&1
  grep -q '^check_unsat: Certificate check passed$' "$evidence/$log"
  grep -q '^check: valid witness$' "$evidence/$log"
}

verify_unsafe() {
  local model=$1 trace=$2 log=$3
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
    -v "$evidence:/out:ro" "$certifaiger_image" \
    /tools/aigsim -c -m "/models/$model" "/out/$trace" \
    >"$evidence/$log" 2>&1
}

reject_safe() {
  local model=$1 certificate=$2 log=$3
  if docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
    -v "$evidence:/out:ro" "$certifaiger_image" \
    /tools/check "/models/$model" "/out/$certificate" \
    >"$evidence/$log" 2>&1; then
    echo "hostile SAFE evidence unexpectedly verified: $log" >&2
    exit 1
  fi
}

reject_unsafe() {
  local model=$1 trace=$2 log=$3
  if docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
    -v "$evidence:/out:ro" "$certifaiger_image" \
    /tools/aigsim -c -m "/models/$model" "/out/$trace" \
    >"$evidence/$log" 2>&1; then
    echo "hostile UNSAFE evidence unexpectedly replayed: $log" >&2
    exit 1
  fi
}

printf '%s\n' \
  'schema_version,horizon,property,expected_answer,external_answer,earliest_bad_frame,model_bytes,evidence_bytes,deterministic,independently_verified,status' \
  >"$scratch/result.csv"

for horizon in 4 5 7 9; do
  for property in wake bark bite; do
    model=h${horizon}-${property}.aag
    first=h${horizon}-${property}.evidence.aag
    second=h${horizon}-${property}.second.aag
    produce "$model" "$first" "h${horizon}-${property}.producer.log"
    produce "$model" "$second" "h${horizon}-${property}.second.log"
    cmp "$evidence/$first" "$evidence/$second"
    actual=$(grep -E '^(SAT|UNSAT)$' \
      "$evidence/h${horizon}-${property}.producer.log" | tail -1)
    expected=$(expected_answer "$horizon" "$property")
    frame=none
    if [[ "$expected" == SAFE ]]; then
      [[ "$actual" == UNSAT ]]
      verify_safe "$model" "$first" "h${horizon}-${property}.consumer.log"
    else
      [[ "$actual" == SAT ]]
      frame=$(expected_frame "$property")
      trace_values=$(awk 'NR >= 4 && $0 != "." { count++ } END { print count + 0 }' \
        "$evidence/$first")
      [[ "$trace_values" -eq $((frame + 1)) ]]
      verify_unsafe "$model" "$first" "h${horizon}-${property}.consumer.log"
    fi
    printf '1,%s,%s,%s,%s,%s,%s,%s,true,true,validated\n' \
      "$horizon" "$property" "$expected" "$actual" "$frame" \
      "$(wc -c <"$models/$model" | tr -d ' ')" \
      "$(wc -c <"$evidence/$first" | tr -d ' ')" \
      >>"$scratch/result.csv"
  done
done
echo "opentitan composed-witness phase=individual-evidence-verified"

"$gcc_binary" compose-safety-witnesses-v1 "$models/h4-safe-set.aag" \
  "$evidence/h4-composed.aag" "$evidence/h4-wake.evidence.aag" \
  "$evidence/h4-bark.evidence.aag" "$evidence/h4-bite.evidence.aag" >/dev/null
"$gcc_binary" compose-safety-witnesses-v1 "$models/h4-safe-set.aag" \
  "$evidence/h4-composed-second.aag" "$evidence/h4-wake.evidence.aag" \
  "$evidence/h4-bark.evidence.aag" "$evidence/h4-bite.evidence.aag" >/dev/null
cmp "$evidence/h4-composed.aag" "$evidence/h4-composed-second.aag"
verify_safe h4-safe-set.aag h4-composed.aag h4-composed.consumer.log

"$gcc_binary" compose-safety-witnesses-v1 "$models/h5-safe-set.aag" \
  "$evidence/h5-composed.aag" "$evidence/h5-wake.evidence.aag" \
  "$evidence/h5-bite.evidence.aag" >/dev/null
"$gcc_binary" compose-safety-witnesses-v1 "$models/h5-safe-set.aag" \
  "$evidence/h5-composed-second.aag" "$evidence/h5-wake.evidence.aag" \
  "$evidence/h5-bite.evidence.aag" >/dev/null
cmp "$evidence/h5-composed.aag" "$evidence/h5-composed-second.aag"
verify_safe h5-safe-set.aag h5-composed.aag h5-composed.consumer.log
echo "opentitan composed-witness phase=compositions-verified"

# Evidence substitution, corruption, and truncation must fail independently.
sed '1s/^aag/bag/' "$evidence/h4-wake.evidence.aag" \
  >"$evidence/h4-wake.malformed.aag"
safe_bytes=$(wc -c <"$evidence/h4-wake.evidence.aag" | tr -d ' ')
dd if="$evidence/h4-wake.evidence.aag" \
  of="$evidence/h4-wake.truncated.aag" bs=1 count=$((safe_bytes - 1)) \
  2>/dev/null
trace_bytes=$(wc -c <"$evidence/h5-bark.evidence.aag" | tr -d ' ')
dd if="$evidence/h5-bark.evidence.aag" \
  of="$evidence/h5-bark.truncated.aag" bs=1 count=$((trace_bytes - 1)) \
  2>/dev/null
reject_safe h4-wake.aag h4-wake.malformed.aag hostile-safe-malformed.log
reject_safe h4-wake.aag h4-wake.truncated.aag hostile-safe-truncated.log
reject_safe h5-wake.aag h4-wake.evidence.aag hostile-safe-wrong-model.log
reject_safe h5-safe-set.aag h4-composed.aag hostile-composed-wrong-model.log
reject_unsafe h5-bark.aag h5-bark.truncated.aag hostile-trace-truncated.log
reject_unsafe h4-bark.aag h5-bark.evidence.aag hostile-trace-wrong-model.log
echo "opentitan composed-witness phase=hostile-controls-rejected"

{
  printf 'schema_version=1\n'
  printf 'scope=opentitan-aon-dual-timer-bounded-identical-scope\n'
  printf 'model_count=15\n'
  printf 'answer_count=12\n'
  printf 'safe_certificate_count=6\n'
  printf 'unsafe_trace_count=6\n'
  printf 'composed_safe_set_count=2\n'
  printf 'hostile_control_count=6\n'
  printf 'aiger_models_sha256=%s\n' "$(sha256sum_portable "$models/SHA256SUMS")"
  printf 'h4_safe_set_model_bytes=%s\n' "$(wc -c \
    <"$models/h4-safe-set.aag" | tr -d ' ')"
  printf 'h5_safe_set_model_bytes=%s\n' "$(wc -c \
    <"$models/h5-safe-set.aag" | tr -d ' ')"
  printf 'h4_individual_evidence_bytes=%s\n' "$(wc -c \
    "$evidence/h4-wake.evidence.aag" "$evidence/h4-bark.evidence.aag" \
    "$evidence/h4-bite.evidence.aag" | awk 'END {print $1}')"
  printf 'h4_composed_evidence_bytes=%s\n' "$(wc -c \
    <"$evidence/h4-composed.aag" | tr -d ' ')"
  printf 'h5_individual_safe_evidence_bytes=%s\n' "$(wc -c \
    "$evidence/h5-wake.evidence.aag" "$evidence/h5-bite.evidence.aag" \
    | awk 'END {print $1}')"
  printf 'h5_composed_evidence_bytes=%s\n' "$(wc -c \
    <"$evidence/h5-composed.aag" | tr -d ' ')"
  printf 'h4_composed_sha256=%s\n' \
    "$(sha256sum_portable "$evidence/h4-composed.aag")"
  printf 'h5_composed_sha256=%s\n' \
    "$(sha256sum_portable "$evidence/h5-composed.aag")"
  printf 'qualification_lock_sha256=%s\n' \
    "$(sha256sum_portable "$repository/tools/certifaiger-qualification-v1.lock")"
  printf 'ric3_binary_sha256=%s\n' \
    "$(sha256sum_portable "$ric3_output/ric3")"
  printf 'certifaiger_tree_sha256=%s\n' \
    "$(tree_sha256_portable "$certifaiger_output/bin")"
  printf 'status=validated\n'
} >"$scratch/manifest.txt"

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
if ! (set -C; cat "$scratch/manifest.txt" >"$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi
echo "opentitan_dual_timer_composed_witness=VALIDATED answers=12 safe=6 unsafe=6 composed=2 output=$output"
