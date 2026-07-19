#!/usr/bin/env sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /path/to/sby.py" >&2
  exit 2
fi

sby=$1
test -f "$sby"
repository=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
source_corpus=$repository/corpus/rtl/wmcontroller
work=$(mktemp -d "${TMPDIR:-/tmp}/gcc-wm-physical-oracle.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
cp -R "$source_corpus" "$work/corpus"
corpus=$work/corpus

check_digests() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c SHA256SUMS
  else
    sha256sum -c SHA256SUMS
  fi
}

(
  cd "$corpus"
  check_digests
  case "$(yosys -V)" in
    "Yosys 0.67+post (git sha1 b8e7da6f40ae8f552c116bf6c359b07c6533e159,"*)
      yosys -Q -q -s synthesize.ys
      check_digests
      echo "controller-aiger-regeneration=PASS yosys=0.67+post-b8e7da6f40ae"
      ;;
    *)
      echo "controller-aiger-regeneration=SKIPPED reason=yosys-version-mismatch"
      ;;
  esac
)
(
  cd "$corpus/plant"
  check_digests
  case "$(yosys -V)" in
    "Yosys 0.67+post (git sha1 b8e7da6f40ae8f552c116bf6c359b07c6533e159,"*)
      yosys -Q -q -s synthesize.ys
      check_digests
      echo "plant-aiger-regeneration=PASS yosys=0.67+post-b8e7da6f40ae"
      ;;
    *)
      echo "plant-aiger-regeneration=SKIPPED reason=yosys-version-mismatch"
      ;;
  esac
)

set +e
(
  cd "$corpus"
  python3 "$sby" -f -d "$work/oracle" physical-plant-monitor.sby
)
oracle_exit=$?
set -e

test "$oracle_exit" -eq 2
test -f "$work/oracle/FAIL"
log=$work/oracle/logfile.txt
grep -Eq 'failed assertion .*assert_door_water .* step 4$' "$log"
grep -Eq 'failed assertion .*assert_overfill .* step 7$' "$log"
grep -Eq 'failed assertion .*assert_unbalanced_spin .* step 15$' "$log"
grep -Eq 'failed assertion .*assert_motor_spin .* step 15$' "$log"
if grep -Eq 'failed assertion .*assert_fault_actuation' "$log"; then
  echo "expected-SAFE fault-actuation assertion failed" >&2
  exit 1
fi
if grep -Eq 'failed assertion .*assert_conflicting_actions' "$log"; then
  echo "expected-SAFE conflicting-actions assertion failed" >&2
  exit 1
fi
grep -q 'Checking assertions in step 32' "$log"

echo "public-washing-physical-oracle=PASS unsafe=4 safe=2 depth=32"
