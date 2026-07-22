#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 YOSYS GCC_BINARY OUTPUT.csv MANIFEST.txt WORKDIR" >&2
  exit 2
fi

yosys=$1
binary=$2
output=$3
manifest=$4
workdir=$5
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)

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

environment=$workdir/environment.btor2
before=$workdir/before.btor2
after=$workdir/after.btor2
interface=$workdir/interface.txt
queries=$workdir/queries.txt
first=$workdir/first.revision-impact
second=$workdir/second.revision-impact

"$repo/scripts/build-opentitan-prim-count-query-service-v1.sh" \
  "$yosys" "$environment" "$before" "$after" >"$workdir/build.log"

printf '%s\n' \
  'word_interface_version=2' \
  'wire_count=6' \
  'wire=left,3,4' \
  'wire=left,3,6' \
  'wire=left,3,7' \
  'wire=left,4,8' \
  'wire=left,9,2' \
  'wire=left,12,5' \
  'external_count=1' \
  'external=left,2' \
  'status=complete' >"$interface"

printf '%s\n' \
  'gcc-btor2-revision-impact-queries-v1' \
  '0,left,2' \
  '0,right,1000' \
  '0,right,1001' \
  '0,right,1003' >"$queries"

run_check() {
  local artifact=$1
  local log=$2
  local metrics=$3
  run_timed "$log" "$metrics" "$binary" check-btor2-revision-impact \
    "$environment" "$environment" 2,3,4,9,12 \
    "$before" "$after" 1000,1001,1002,1003,1004,1005,1006,1007 \
    "$interface" "$interface" "$queries" "$artifact" >"$log"
}

run_check "$first" "$workdir/first.log" "$workdir/first.time"
run_timed "$workdir/verify.log" "$workdir/verify.time" \
  "$binary" verify-btor2-revision-impact \
  "$environment" "$environment" 2,3,4,9,12 \
  "$before" "$after" 1000,1001,1002,1003,1004,1005,1006,1007 \
  "$interface" "$interface" "$queries" "$first"
run_check "$second" "$workdir/second.log" "$workdir/second.time"

cmp "$first" "$second"
first_logical=$(sed -E '1s/ elapsed_micros=[0-9]+$//' "$workdir/first.log")
second_logical=$(sed -E '1s/ elapsed_micros=[0-9]+$//' "$workdir/second.log")
[[ "$first_logical" == "$second_logical" ]] || {
  echo "logical output or deterministic work counters differ" >&2
  exit 2
}

expected_transitions=(
  'btor2-revision-impact-query index=0 horizon=0 bad_side=left bad_output=2 old_result=UNSAFE new_result=UNSAFE'
  'btor2-revision-impact-query index=1 horizon=0 bad_side=right bad_output=1000 old_result=UNSAFE new_result=SAFE'
  'btor2-revision-impact-query index=2 horizon=0 bad_side=right bad_output=1001 old_result=SAFE new_result=SAFE'
  'btor2-revision-impact-query index=3 horizon=0 bad_side=right bad_output=1003 old_result=SAFE new_result=UNSAFE'
)
for transition in "${expected_transitions[@]}"; do
  grep -Fxq "$transition" "$workdir/first.log"
  grep -Fxq "$transition" "$workdir/verify.log"
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
certificate_sha256=$(sha256_file "$first")
read_timing "$workdir/first.time"
producer_elapsed_seconds=$timing_elapsed
producer_peak_rss_bytes=$timing_peak_bytes
read_timing "$workdir/verify.time"
checker_elapsed_seconds=$timing_elapsed
checker_peak_rss_bytes=$timing_peak_bytes

{
  printf '%s\n' 'schema_version,query_index,horizon,bad_side,bad_output,old_result,new_result,transition_class,certificate_bytes,parsed_evidence_bytes,semantic_replays,component_validations,composed_pair_checks,final_transition_checks,result_comparisons,producer_elapsed_seconds,checker_elapsed_seconds,producer_peak_rss_bytes,checker_peak_rss_bytes,time_backend,certificate_sha256,deterministic,status'
  printf '1,0,0,left,2,UNSAFE,UNSAFE,unchanged-unsafe,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' "$certificate_bytes" "$parsed_evidence_bytes" "$semantic_replays" "$component_validations" "$composed_pair_checks" "$final_transition_checks" "$result_comparisons" "$producer_elapsed_seconds" "$checker_elapsed_seconds" "$producer_peak_rss_bytes" "$checker_peak_rss_bytes" "$time_style" "$certificate_sha256"
  printf '1,1,0,right,1000,UNSAFE,SAFE,unsafe-to-safe,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' "$certificate_bytes" "$parsed_evidence_bytes" "$semantic_replays" "$component_validations" "$composed_pair_checks" "$final_transition_checks" "$result_comparisons" "$producer_elapsed_seconds" "$checker_elapsed_seconds" "$producer_peak_rss_bytes" "$checker_peak_rss_bytes" "$time_style" "$certificate_sha256"
  printf '1,2,0,right,1001,SAFE,SAFE,unchanged-safe,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' "$certificate_bytes" "$parsed_evidence_bytes" "$semantic_replays" "$component_validations" "$composed_pair_checks" "$final_transition_checks" "$result_comparisons" "$producer_elapsed_seconds" "$checker_elapsed_seconds" "$producer_peak_rss_bytes" "$checker_peak_rss_bytes" "$time_style" "$certificate_sha256"
  printf '1,3,0,right,1003,SAFE,UNSAFE,safe-to-unsafe,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' "$certificate_bytes" "$parsed_evidence_bytes" "$semantic_replays" "$component_validations" "$composed_pair_checks" "$final_transition_checks" "$result_comparisons" "$producer_elapsed_seconds" "$checker_elapsed_seconds" "$producer_peak_rss_bytes" "$checker_peak_rss_bytes" "$time_style" "$certificate_sha256"
} >"$workdir/result.csv"

{
  printf 'schema_version=1\n'
  printf 'yosys_version=%s\n' "$("$yosys" -V)"
  printf 'gcc_capabilities=%s\n' "$("$binary" btor2-revision-impact-cli-version)"
  for path in "$environment" "$before" "$after" "$interface" "$queries" "$first" "$second"; do
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

echo "opentitan_prim_count_revision_impact_v1=PASS transitions=4 deterministic=true certificate_bytes=$certificate_bytes output=$output"
