#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 gcc-produce|gcc-consume|external-produce|external-consume 4|5 REPETITIONS" >&2
  exit 2
}
mode=$1
horizon=$2
repetitions=$3
[[ "$horizon" == 4 || "$horizon" == 5 ]] || {
  echo "resource comparison horizon must be 4 or 5" >&2
  exit 2
}
[[ "$repetitions" =~ ^[1-9][0-9]*$ && "$repetitions" -le 20 ]] || {
  echo "resource repetitions must be in 1..=20" >&2
  exit 2
}
if [[ "$horizon" == 4 ]]; then
  expected_answers='33:SAFE:none,37:SAFE:none,41:SAFE:none'
else
  expected_answers='33:SAFE:none,37:UNSAFE:5,41:SAFE:none'
fi

produce_one() {
  local directory=$1 property=$2 expected=$3
  /tools/ric3 check "/models/h${horizon}-${property}.aag" \
    --cert "$directory/h${horizon}-${property}.evidence.aag" --ui false ic3 \
    >"$directory/h${horizon}-${property}.producer.log" 2>&1
  grep -qx "$expected" "$directory/h${horizon}-${property}.producer.log"
}

case "$mode" in
  gcc-*) prefix=gcc-repeat ;;
  external-*) prefix=external-repeat ;;
  *) echo "invalid resource comparison mode" >&2; exit 2 ;;
esac

for repetition in $(seq 1 "$repetitions"); do
  directory=/out/$prefix-$repetition
  case "$mode" in
    gcc-produce)
      mkdir "$directory"
      /gcc/guarded-continuation-checker check-btor2-predicate-set \
        /repo/corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2 \
        33,37,41 "$horizon" "$directory/gcc.cert" \
        >"$directory/gcc-producer.log"
      grep -q 'route=invariant-chained-regions' "$directory/gcc-producer.log"
      grep -q "answers=$expected_answers" "$directory/gcc-producer.log"
      ;;
    gcc-consume)
      /gcc/guarded-continuation-checker verify-btor2-predicate-set \
        /repo/corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2 \
        33,37,41 "$horizon" "$directory/gcc.cert" \
        >"$directory/gcc-consumer.log"
      grep -q 'route=invariant-chained-regions' "$directory/gcc-consumer.log"
      grep -q "answers=$expected_answers" "$directory/gcc-consumer.log"
      ;;
    external-produce)
      mkdir "$directory"
      if [[ "$horizon" == 4 ]]; then
        produce_one "$directory" wake UNSAT
        produce_one "$directory" bark UNSAT
        produce_one "$directory" bite UNSAT
        /gcc/guarded-continuation-checker compose-safety-witnesses-v1 \
          /models/h4-safe-set.aag "$directory/h4-composed.aag" \
          "$directory/h4-wake.evidence.aag" \
          "$directory/h4-bark.evidence.aag" \
          "$directory/h4-bite.evidence.aag" >/dev/null
      else
        produce_one "$directory" wake UNSAT
        produce_one "$directory" bark SAT
        produce_one "$directory" bite UNSAT
        /gcc/guarded-continuation-checker compose-safety-witnesses-v1 \
          /models/h5-safe-set.aag "$directory/h5-composed.aag" \
          "$directory/h5-wake.evidence.aag" \
          "$directory/h5-bite.evidence.aag" >/dev/null
      fi
      ;;
    external-consume)
      if [[ "$horizon" == 4 ]]; then
        /cert/check /models/h4-safe-set.aag "$directory/h4-composed.aag"
      else
        /cert/check /models/h5-safe-set.aag "$directory/h5-composed.aag"
        /cert/aigsim -c -m /models/h5-bark.aag \
          "$directory/h5-bark.evidence.aag"
      fi
      ;;
  esac
done
