#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 YOSYS Z3 GCC_COHORT_EXAMPLE OUTPUT_DIRECTORY" >&2
  exit 2
fi

yosys=$1
z3=$2
cohort=$3
output=$4
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159
expected_z3='Z3 version 4.16.0'

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

[[ -x $yosys && -x $z3 && -x $cohort ]] || {
  echo "Yosys, Z3, and the GCC cohort executable must be executable" >&2
  exit 2
}
[[ $($yosys -V) == *"git sha1 $expected_yosys,"* ]] || {
  echo "Yosys revision mismatch" >&2
  exit 2
}
[[ $($z3 --version) == "$expected_z3"* ]] || {
  echo "Z3 version mismatch" >&2
  exit 2
}
[[ ! -e $output && ! -L $output ]] || {
  echo "refusing to overwrite output directory" >&2
  exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-pwm-trace-baseline.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir "$scratch/result"

GCC_PWM_HARNESS="$repo/corpus/rtl/opentitan-pwm-channel-family/symbolic-class-harness.sv" \
GCC_PWM_TOP=opentitan_pwm_symbolic_class_harness \
GCC_PWM_OUTPUT_FORMAT=smt2 \
  "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  "$yosys" "$scratch/2.smt2" "$scratch/4.smt2" "$scratch/6.smt2" \
  >"$scratch/yosys.log" 2>&1

"$cohort" >"$scratch/result/gcc-results.csv"
printf '%s\n' \
  'model,query_id,channel,length,mask,value,horizon,result,bad_frame' \
  >"$scratch/result/maintained-results.csv"

top=opentitan_pwm_symbolic_class_harness
shapes=(
  '1 1 1 1'
  '1 1 0 1'
  '2 3 1 8'
  '2 3 2 8'
  '3 7 2 8'
  '3 7 5 8'
  '3 5 1 8'
)

for channels in 2 4 6; do
  model="symbolic-class-$channels"
  query_smt="$scratch/query-$channels.smt2"
  transcript="$scratch/transcript-$channels.txt"
  cp "$scratch/$channels.smt2" "$query_smt"
  for frame in $(seq 0 8); do
    printf '(declare-fun s%s () |%s_s|)\n' "$frame" "$top" >>"$query_smt"
  done
  printf '(assert (|%s_i| s0))\n' "$top" >>"$query_smt"
  for frame in $(seq 0 8); do
    printf '(assert (|%s_h| s%s))\n' "$top" "$frame" >>"$query_smt"
    printf '(assert (|%s_a| s%s))\n' "$top" "$frame" >>"$query_smt"
    printf '(assert (|%s_u| s%s))\n' "$top" "$frame" >>"$query_smt"
    if (( frame < 8 )); then
      printf '(assert (|%s_t| s%s s%s))\n' "$top" "$frame" "$((frame + 1))" \
        >>"$query_smt"
    fi
  done

  query_id=0
  for shape in "${shapes[@]}"; do
    read -r length mask value horizon <<<"$shape"
    for channel in $(seq 0 "$((channels - 1))"); do
      printf '(echo "QUERY,%s,%s,%s,%s,%s,%s,%s")\n' \
        "$model" "$query_id" "$channel" "$length" "$mask" "$value" "$horizon" \
        >>"$query_smt"
      for frame in $(seq "$((length - 1))" "$horizon"); do
        printf '(echo "FRAME,%s")\n(push 1)\n(assert (and' "$frame" >>"$query_smt"
        for bit in $(seq 0 "$((length - 1))"); do
          if (( (mask >> bit) & 1 )); then
            expected=$(((value >> bit) & 1))
            state=$((frame - bit))
            printf ' (= ((_ extract %s %s) (|%s_n pwm_o| s%s)) #b%s)' \
              "$channel" "$channel" "$top" "$state" "$expected" >>"$query_smt"
          fi
        done
        printf '))\n(check-sat)\n(pop 1)\n' >>"$query_smt"
      done
      printf '(echo "END")\n' >>"$query_smt"
      query_id=$((query_id + 1))
    done
  done
  "$z3" "$query_smt" >"$transcript"
  awk -F, '
    /^QUERY,/ {
      row = substr($0, 7)
      result = "SAFE"
      bad = "none"
      next
    }
    /^FRAME,/ { frame = $2; next }
    /^sat$/ {
      if (result == "SAFE") { result = "UNSAFE"; bad = frame }
      next
    }
    /^unsat$/ { next }
    /^unknown$/ { print "Z3 returned unknown" > "/dev/stderr"; exit 2 }
    /^END$/ { print row "," result "," bad; next }
    { print "unexpected Z3 output: " $0 > "/dev/stderr"; exit 2 }
  ' "$transcript" >>"$scratch/result/maintained-results.csv"
done

if ! diff -u "$scratch/result/gcc-results.csv" "$scratch/result/maintained-results.csv" \
  >"$scratch/result/agreement.diff"; then
  cat "$scratch/result/agreement.diff" >&2
  exit 1
fi
if ! diff -u "$repo/results/opentitan-pwm-trace-maintained-v1.csv" \
  "$scratch/result/maintained-results.csv" >"$scratch/result/retained.diff"; then
  cat "$scratch/result/retained.diff" >&2
  exit 1
fi
printf 'yosys_revision=%s\n' "$expected_yosys" >"$scratch/result/tools.txt"
printf 'z3_version=%s\n' "$($z3 --version)" >>"$scratch/result/tools.txt"
printf 'queries=%s\nagreement=PASS\n' \
  "$(( $(wc -l <"$scratch/result/gcc-results.csv") - 1 ))" \
  >>"$scratch/result/tools.txt"
for channels in 2 4 6; do
  printf '%s  symbolic-class-%s.smt2\n' \
    "$(sha256_file "$scratch/$channels.smt2")" "$channels" \
    >>"$scratch/result/smt2-sha256.txt"
done

mv "$scratch/result" "$output"
echo "opentitan_pwm_trace_maintained_baseline_v1=PASS queries=84 output=$output"
