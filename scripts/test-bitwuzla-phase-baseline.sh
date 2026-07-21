#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 BITWUZLA_BINARY" >&2
  exit 2
fi

bitwuzla=$1
test -x "$bitwuzla"

check_result() {
  expected=$1
  input=$2
  actual=$($bitwuzla --lang smt2 -t 10000 -M 512 "$input")
  if [ "$actual" != "$expected" ]; then
    echo "Bitwuzla result for $input: expected $expected, got $actual" >&2
    exit 1
  fi
}

check_result sat examples/btor2/watchdog-endpoint-v1.smt2
check_result unsat examples/btor2/watchdog-endpoint-tampered-v1.smt2
check_result sat examples/btor2/actuator-endpoint-v1.smt2
check_result unsat examples/btor2/watchdog-search-h2-safe-v1.smt2
check_result sat examples/btor2/watchdog-search-h3-unsafe-v1.smt2
check_result unsat examples/btor2/actuator-region-h200-safe-v1.smt2
check_result sat examples/btor2/actuator-region-h201-unsafe-v1.smt2
check_result unsat examples/btor2/saturating-region-h254-safe-v1.smt2
check_result sat examples/btor2/saturating-region-h255-unsafe-v1.smt2
check_result unsat examples/btor2/motion-curve-h200-safe-v1.smt2
check_result sat examples/btor2/motion-curve-h201-unsafe-v1.smt2
check_result unsat examples/btor2/servo-curve-h128-safe-v1.smt2
check_result sat examples/btor2/servo-curve-h129-unsafe-v1.smt2
check_result unsat examples/btor2/braking-phases-h255-safe-v1.smt2
check_result sat examples/btor2/braking-phases-h256-unsafe-v1.smt2
check_result unsat examples/btor2/motor-stop-h159-safe-v1.smt2
check_result sat examples/btor2/motor-stop-h160-unsafe-v1.smt2
check_result unsat corpus/rtl/opentitan-aon-timer/baselines/small-h8-safe.smt2
check_result sat corpus/rtl/opentitan-aon-timer/baselines/small-h9-unsafe.smt2
check_result unsat corpus/rtl/opentitan-aon-timer/baselines/scale-h1000000000-safe.smt2
check_result unsat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-small-h4-safe.smt2
check_result sat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-small-h5-bark-unsafe.smt2
check_result unsat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-small-h5-bite-safe.smt2
check_result sat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-small-h1000000000-bark-unsafe.smt2
check_result sat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-small-h1000000000-bite-unsafe.smt2
check_result unsat corpus/rtl/opentitan-aon-timer/baselines/predicate-set-scale-h1000000000-safe.smt2

version=$($bitwuzla --version)
printf 'bitwuzla_phase_baseline=PASS version=%s\n' "$version"
