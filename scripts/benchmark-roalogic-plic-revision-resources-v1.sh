#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 YOSYS GCC_BINARY OUTPUT.csv WORKDIR TRIALS" >&2
  exit 2
fi

yosys=$1
binary=$2
output=$3
workdir=$4
trials=$5
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cohort=$repo/corpus/rtl/roalogic-plic-gateway/revision-cohort
revision=2e8dc667f6ab69befaebdc30de7a9a53e925dbcc

[[ -x "$yosys" && -x "$binary" ]] || {
  echo "Yosys and GCC must be executable" >&2
  exit 2
}
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}
case "$trials" in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
(( trials <= 20 )) || { echo "TRIALS exceeds limit 20" >&2; exit 2; }

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)
plic=$workdir/plic.btor2
"$repo/scripts/build-roalogic-plic-revision-component-v1.sh" \
  "$yosys" "$revision" "$plic" >/dev/null

previous=$workdir/previous.revision-proof
reference=$workdir/reference.revision-proof
"$binary" check-btor2-revision-portfolio \
  "$plic" 13 "$cohort/monitor.btor2" 7,8 \
  "$cohort/interface-retained-plic.txt" 2 right 7 "$previous" >/dev/null
"$binary" check-btor2-revision-portfolio \
  "$plic" 13 "$cohort/monitor-v2.btor2" 8,9 \
  "$cohort/interface-retained-plic.txt" 2 right 8 "$reference" >/dev/null
artifact_bytes=$(wc -c <"$reference" | tr -d ' ')

printf '%s\n' \
  'schema_version,operation,trial,elapsed_seconds,peak_rss_bytes,artifact_bytes,time_backend,platform,result,verified_local_sections,reused_local_sections,status' \
  >"$output"

measure() {
  operation=$1
  trial=$2
  shift 2
  stdout=$workdir/$operation-$trial.stdout
  metrics=$workdir/$operation-$trial.time
  if [[ $time_style == bsd ]]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  [[ -n $elapsed && -n $peak_bytes && $peak_bytes -gt 0 ]]
  grep -q 'result=UNSAFE.*bad_frame=2' "$stdout"
  case "$operation" in
    full-create | full-verify)
      verified=2
      reused=0
      ;;
    retained-create)
      grep -q 'produced_local_sections=1 production_reused_local_sections=1' "$stdout"
      verified=1
      reused=1
      ;;
    retained-verify)
      grep -q 'verified_local_sections=1 reused_local_sections=1' "$stdout"
      verified=1
      reused=1
      ;;
  esac
  printf '1,%s,%s,%s,%s,%s,%s,%s,UNSAFE,%s,%s,measured\n' \
    "$operation" "$trial" "$elapsed" "$peak_bytes" "$artifact_bytes" \
    "$time_style" "$platform" "$verified" "$reused" >>"$output"
}

trial=1
while (( trial <= trials )); do
  full_output=$workdir/full-$trial.revision-proof
  retained_output=$workdir/retained-$trial.revision-proof
  if (( trial % 2 == 1 )); then
    measure full-create "$trial" "$binary" check-btor2-revision-portfolio \
      "$plic" 13 "$cohort/monitor-v2.btor2" 8,9 \
      "$cohort/interface-retained-plic.txt" 2 right 8 "$full_output"
    measure retained-create "$trial" "$binary" check-btor2-revision-retained-left \
      "$plic" "$previous" "$cohort/monitor-v2.btor2" 8,9 \
      "$cohort/interface-retained-plic.txt" 2 right 8 "$retained_output"
  else
    measure retained-create "$trial" "$binary" check-btor2-revision-retained-left \
      "$plic" "$previous" "$cohort/monitor-v2.btor2" 8,9 \
      "$cohort/interface-retained-plic.txt" 2 right 8 "$retained_output"
    measure full-create "$trial" "$binary" check-btor2-revision-portfolio \
      "$plic" 13 "$cohort/monitor-v2.btor2" 8,9 \
      "$cohort/interface-retained-plic.txt" 2 right 8 "$full_output"
  fi
  cmp "$full_output" "$retained_output"
  measure full-verify "$trial" "$binary" verify-btor2-revision-portfolio \
    "$plic" 13 "$cohort/monitor-v2.btor2" 8,9 \
    "$cohort/interface-retained-plic.txt" 2 right 8 "$reference"
  measure retained-verify "$trial" "$binary" verify-btor2-revision-retained-left \
    "$plic" "$previous" "$cohort/monitor-v2.btor2" \
    "$cohort/interface-retained-plic.txt" 2 right 8 "$reference"
  trial=$((trial + 1))
done

echo "roalogic_plic_revision_resources_v1=MEASURED trials=$trials operations=4 output=$output"
