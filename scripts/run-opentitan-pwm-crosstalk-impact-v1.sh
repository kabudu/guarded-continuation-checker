#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 YOSYS GCC_BINARY OUTPUT.csv MANIFEST.txt WORKDIR TRIAL" >&2
  exit 2
fi

yosys=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
binary=$(cd "$(dirname "$2")" && pwd -P)/$(basename "$2")
output=$3
manifest=$4
workdir=$5
trial=$6
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-pwm-crosstalk-impact
expected_certificate_sha256=e788c497b514472db64fd79fd5fa319f03abf257a3cd656c96a2eb73a44678b3

[[ -x "$yosys" && -x "$binary" ]] || {
  echo "Yosys and GCC must be executable" >&2
  exit 2
}
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
for path in "$output" "$manifest"; do
  [[ ! -e "$path" && ! -L "$path" ]] || {
    echo "refusing to overwrite $path" >&2
    exit 2
  }
done
case "$trial" in
  ''|*[!0-9]*) echo "TRIAL must be an integer" >&2; exit 2 ;;
esac

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported timing platform" >&2; exit 2 ;;
esac

run_timed() {
  local stdout=$1
  local metrics=$2
  shift 2
  if [[ $time_style == bsd ]]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"
  else
    /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"
  fi
}

read_timing() {
  local metrics=$1
  if [[ $time_style == bsd ]]; then
    timing_elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    timing_peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    read -r timing_elapsed timing_peak_kib <"$metrics"
    timing_peak_bytes=$((timing_peak_kib * 1024))
  fi
  [[ -n $timing_elapsed && -n $timing_peak_bytes && $timing_peak_bytes -gt 0 ]]
}

core_before=$workdir/core-before.btor2
core_after=$workdir/core-after.btor2
channel_before=$workdir/channel-before.btor2
channel_after=$workdir/channel-after.btor2
first=$workdir/first.revision-impact
second=$workdir/second.revision-impact

run_timed "$workdir/build.log" "$workdir/build.time" \
  "$repo/scripts/build-opentitan-pwm-crosstalk-impact-v1.sh" \
  "$yosys" "$core_before" "$core_after" "$channel_before" "$channel_after"
read_timing "$workdir/build.time"
synthesis_elapsed_seconds=$timing_elapsed
synthesis_peak_rss_bytes=$timing_peak_bytes

for name in core-before core-after channel-before channel-after; do
  cmp "$workdir/$name.btor2" "$fixture/generated/$name.btor2"
done

common_args=(
  "$core_before" "$core_after" 1000,1001,1002,1003
  "$channel_before" "$channel_after" 1000,1001,1002,1003,1004
  "$fixture/interface.txt" "$fixture/interface.txt" "$fixture/queries.txt"
)

run_timed "$workdir/first.log" "$workdir/first.time" \
  "$binary" check-btor2-revision-impact "${common_args[@]}" "$first"
run_timed "$workdir/verify.log" "$workdir/verify.time" \
  "$binary" verify-btor2-revision-impact "${common_args[@]}" "$first"
run_timed "$workdir/second.log" "$workdir/second.time" \
  "$binary" check-btor2-revision-impact "${common_args[@]}" "$second"

cmp "$first" "$second"
first_logical=$(sed -E '1s/ elapsed_micros=[0-9]+$//' "$workdir/first.log")
second_logical=$(sed -E '1s/ elapsed_micros=[0-9]+$//' "$workdir/second.log")
[[ "$first_logical" == "$second_logical" ]] || {
  echo "logical output or deterministic work counters differ" >&2
  exit 2
}

expected_lines=(
  'btor2-revision-impact-query index=0 horizon=0 bad_side=right bad_output=1004 old_result=UNSAFE new_result=UNSAFE'
  'btor2-revision-impact-query index=1 horizon=4 bad_side=right bad_output=1000 old_result=UNSAFE new_result=SAFE'
  'btor2-revision-impact-query index=2 horizon=4 bad_side=right bad_output=1001 old_result=UNSAFE new_result=SAFE'
  'btor2-revision-impact-query index=3 horizon=4 bad_side=right bad_output=1002 old_result=UNSAFE new_result=SAFE'
  'btor2-revision-impact-query index=4 horizon=4 bad_side=right bad_output=1003 old_result=SAFE new_result=SAFE'
  'btor2-revision-impact-semantic-set query_index=1 changed_mask=1 baseline_result=UNSAFE changed_result=SAFE'
  'btor2-revision-impact-semantic-set query_index=2 changed_mask=2 baseline_result=UNSAFE changed_result=SAFE'
  'btor2-revision-impact-semantic-set query_index=3 changed_mask=3 baseline_result=UNSAFE changed_result=SAFE'
)
for expected in "${expected_lines[@]}"; do
  grep -Fxq "$expected" "$workdir/first.log"
  grep -Fxq "$expected" "$workdir/verify.log"
done

summary=$(head -n 1 "$workdir/first.log")
field() {
  local key=$1
  printf '%s\n' "$summary" | tr ' ' '\n' | sed -n "s/^${key}=//p"
}
certificate_bytes=$(field certificate_bytes)
parsed_evidence_bytes=$(field parsed_evidence_bytes)
semantic_replays=$(field semantic_replays)
component_validations=$(field component_validations)
composed_pair_checks=$(field composed_pair_checks)
final_transition_checks=$(field final_transition_checks)
result_comparisons=$(field result_comparisons)
producer_internal_micros=$(field elapsed_micros)
verify_summary=$(head -n 1 "$workdir/verify.log")
checker_internal_micros=$(printf '%s\n' "$verify_summary" | tr ' ' '\n' | sed -n 's/^elapsed_micros=//p')
certificate_sha256=$(sha256_file "$first")
[[ $certificate_bytes -eq 128768 ]]
[[ $certificate_sha256 == "$expected_certificate_sha256" ]]
[[ $(field minimal_semantic_change_sets) -eq 3 ]]
[[ $semantic_replays -eq 20 && $component_validations -eq 40 && $result_comparisons -eq 20 ]]

read_timing "$workdir/first.time"
producer_elapsed_seconds=$timing_elapsed
producer_peak_rss_bytes=$timing_peak_bytes
read_timing "$workdir/verify.time"
checker_elapsed_seconds=$timing_elapsed
checker_peak_rss_bytes=$timing_peak_bytes
total_producer_elapsed_seconds=$(awk -v synthesis="$synthesis_elapsed_seconds" -v producer="$producer_elapsed_seconds" 'BEGIN { printf "%.2f", synthesis + producer }')
if (( synthesis_peak_rss_bytes > producer_peak_rss_bytes )); then
  total_producer_peak_rss_bytes=$synthesis_peak_rss_bytes
else
  total_producer_peak_rss_bytes=$producer_peak_rss_bytes
fi

printf '%s\n' \
  'schema_version,trial,atoms,combinations,queries,observations,safe,unsafe,minimal_semantic_change_sets,certificate_bytes,parsed_evidence_bytes,semantic_replays,component_validations,composed_pair_checks,final_transition_checks,result_comparisons,producer_internal_micros,checker_internal_micros,synthesis_elapsed_seconds,producer_elapsed_seconds,total_producer_elapsed_seconds,checker_elapsed_seconds,synthesis_peak_rss_bytes,producer_peak_rss_bytes,total_producer_peak_rss_bytes,checker_peak_rss_bytes,time_backend,certificate_sha256,deterministic,status' \
  "1,$trial,2,4,5,20,9,11,3,$certificate_bytes,$parsed_evidence_bytes,$semantic_replays,$component_validations,$composed_pair_checks,$final_transition_checks,$result_comparisons,$producer_internal_micros,$checker_internal_micros,$synthesis_elapsed_seconds,$producer_elapsed_seconds,$total_producer_elapsed_seconds,$checker_elapsed_seconds,$synthesis_peak_rss_bytes,$producer_peak_rss_bytes,$total_producer_peak_rss_bytes,$checker_peak_rss_bytes,$time_style,$certificate_sha256,true,accepted" \
  >"$workdir/result.csv"

{
  printf 'schema_version=1\n'
  printf 'scope=opentitan-pwm-crosstalk-two-atom-five-query-revision-impact\n'
  printf 'yosys_version=%s\n' "$("$yosys" -V)"
  printf 'gcc_capabilities=%s\n' "$("$binary" btor2-revision-impact-cli-version)"
  printf 'gcc_binary_sha256=%s\n' "$(sha256_file "$binary")"
  printf 'timing_scope=source-synthesis-plus-certificate-production\n'
  printf 'memory_scope=max-native-synthesis-or-certificate-process\n'
  for path in "$core_before" "$core_after" "$channel_before" "$channel_after" "$fixture/interface.txt" "$fixture/queries.txt" "$first" "$second"; do
    printf 'sha256=%s file=%s\n' "$(sha256_file "$path")" "$(basename "$path")"
  done
  printf 'status=complete\n'
} >"$workdir/manifest.txt"

if ! (set -C; cp "$workdir/result.csv" "$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
if ! (set -C; cp "$workdir/manifest.txt" "$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi

echo "opentitan_pwm_crosstalk_impact_v1=PASS observations=20 semantic_change_sets=3 deterministic=true certificate_bytes=$certificate_bytes output=$output"
