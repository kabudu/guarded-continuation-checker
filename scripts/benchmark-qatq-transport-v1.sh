#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 6 || $(( $# % 2 )) -ne 0 ]]; then
  echo "usage: $0 BINARY OUTPUT.csv RESOURCES.csv WORKDIR NAME INPUT [NAME INPUT ...]" >&2
  exit 2
fi
binary=$1
output=$2
resources=$3
workdir=$4
fixture_count=$(( ($# - 4) / 2 ))
shift 4
trials=${TRIALS:-5}
chunk_values=${CHUNK_VALUES:-1048576}

[[ -x "$binary" ]] || { echo "qualification binary must be executable" >&2; exit 2; }
[[ -d "$workdir" && ! -L "$workdir" ]] || { echo "WORKDIR must be an existing ordinary directory" >&2; exit 2; }
for target in "$output" "$resources"; do
  [[ ! -e "$target" && ! -L "$target" ]] || { echo "refusing to overwrite $target" >&2; exit 2; }
done
case "$trials" in ''|*[!0-9]*) echo "TRIALS must be an integer" >&2; exit 2;; esac
case "$chunk_values" in ''|*[!0-9]*) echo "CHUNK_VALUES must be an integer" >&2; exit 2;; esac
(( trials >= 5 && trials <= 21 )) || { echo "TRIALS must be between 5 and 21" >&2; exit 2; }
(( chunk_values >= 1 && chunk_values <= 1048576 )) || { echo "CHUNK_VALUES must be between 1 and 1048576" >&2; exit 2; }
command -v zstd >/dev/null 2>&1 || { echo "zstd is required" >&2; exit 2; }

printf '%s\n' 'schema_version,fixture,os,arch,raw_bytes,envelope_bytes,qatq_payload_bytes,ratio_to_raw,max_values_per_chunk,trials,encode_min_ns,encode_median_ns,encode_max_ns,decode_min_ns,decode_median_ns,decode_max_ns,canonical_sha256,envelope_sha256,deterministic,bit_identical,zstd_22_long_27_bytes,qatq_smaller_than_zstd_percent,status' >"$workdir/result.csv"
printf '%s\n' 'schema_version,fixture,os,arch,peak_resident_bytes,measurement,status' >"$workdir/resources.csv"

while (( $# > 0 )); do
  name=$1
  input=$2
  shift 2
  [[ $name =~ ^[a-z0-9][a-z0-9-]*$ ]] || { echo "invalid fixture name $name" >&2; exit 2; }
  [[ -f "$input" && ! -L "$input" ]] || { echo "fixture must be an ordinary file: $input" >&2; exit 2; }
  row=$workdir/$name.csv
  timing=$workdir/$name.time
  zstd_file=$workdir/$name.zst
  decoded=$workdir/$name.decoded
  for target in "$row" "$timing" "$zstd_file" "$decoded"; do
    [[ ! -e "$target" && ! -L "$target" ]] || { echo "work target already exists: $target" >&2; exit 2; }
  done

  if [[ $(uname -s) == Darwin ]]; then
    /usr/bin/time -l "$binary" "$input" "$row" "$trials" "$chunk_values" 2>"$timing"
    peak=$(awk '/maximum resident set size/ { print $1 }' "$timing")
  else
    /usr/bin/time -v "$binary" "$input" "$row" "$trials" "$chunk_values" 2>"$timing"
    peak_kib=$(awk -F: '/Maximum resident set size/ { gsub(/^[[:space:]]+/, "", $2); print $2 }' "$timing")
    peak=$((peak_kib * 1024))
  fi
  [[ $peak =~ ^[0-9]+$ && $peak -gt 0 ]] || { echo "missing peak resident measurement for $name" >&2; exit 2; }

  zstd -q -f -22 --long=27 "$input" -o "$zstd_file"
  zstd -q -d -f "$zstd_file" -o "$decoded"
  cmp "$input" "$decoded"
  zstd_bytes=$(wc -c <"$zstd_file" | tr -d ' ')
  qatq_bytes=$(awk -F, 'NR == 2 { print $5 }' "$row")
  smaller=$(awk -v q="$qatq_bytes" -v z="$zstd_bytes" 'BEGIN { printf "%.6f", 100.0 * (z - q) / z }')
  awk -F, -v name="$name" -v z="$zstd_bytes" -v smaller="$smaller" 'NR == 2 { printf "1,%s", name; for (i = 2; i <= NF - 1; i++) printf ",%s", $i; printf ",%s,%s,%s\n", z, smaller, $NF }' "$row" >>"$workdir/result.csv"
  printf '1,%s,%s,%s,%s,process-peak-rss,measured\n' "$name" "$(uname -s | tr '[:upper:]' '[:lower:]')" "$(uname -m)" "$peak" >>"$workdir/resources.csv"
done

batch_row=$(awk -F, '$2 == "revision-batch" { print $0 }' "$workdir/result.csv")
if [[ -n $batch_row ]]; then
  batch_envelope=$(printf '%s\n' "$batch_row" | awk -F, '{ print $6 }')
  (( batch_envelope * 10 <= 116769 * 9 )) || {
    echo "revision batch failed predeclared 10 percent zstd advantage" >&2
    exit 1
  }
fi

(set -C; cp "$workdir/result.csv" "$output") 2>/dev/null || { echo "refusing to overwrite $output" >&2; exit 2; }
(set -C; cp "$workdir/resources.csv" "$resources") 2>/dev/null || { echo "refusing to overwrite $resources" >&2; exit 2; }
echo "qatq_transport_v1=MEASURED fixtures=$fixture_count output=$output resources=$resources"
