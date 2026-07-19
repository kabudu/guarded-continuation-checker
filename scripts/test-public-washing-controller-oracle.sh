#!/usr/bin/env sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /path/to/sby.py" >&2
  exit 2
fi

sby=$1
if [ ! -f "$sby" ]; then
  echo "SymbiYosys entry point not found: $sby" >&2
  exit 2
fi

repository=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
source_dir="$repository/corpus/rtl/wmcontroller"
work=$(mktemp -d "${TMPDIR:-/tmp}/gcc-wm-oracle.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
cp -R "$source_dir" "$work/corpus"

cd "$work/corpus"

check_digests() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c SHA256SUMS
  else
    sha256sum -c SHA256SUMS
  fi
}

check_digests
case "$(yosys -V)" in
  "Yosys 0.67+post (git sha1 b8e7da6f40ae8f552c116bf6c359b07c6533e159,"*)
    cp generated/controller.aag "$work/controller.before.aag"
    yosys -Q -q -s synthesize.ys
    check_digests
    cmp "$work/controller.before.aag" generated/controller.aag
    echo "controller-aiger-regeneration=PASS yosys=0.67+post-b8e7da6f40ae"
    ;;
  *)
    echo "controller-aiger-regeneration=SKIPPED reason=yosys-version-mismatch"
    ;;
esac

python3 "$sby" -f -d "$work/safe" safe-monitor.sby
test -f "$work/safe/PASS"

if python3 "$sby" -f -d "$work/unsafe" unsafe-monitor.sby; then
  echo "unsafe washing-controller oracle unexpectedly passed" >&2
  exit 1
fi
test -f "$work/unsafe/FAIL"
if ! grep -Eq 'failed assertion.*step 10' "$work/unsafe/logfile.txt"; then
  echo "unsafe washing-controller oracle did not fail at expected step 10" >&2
  exit 1
fi

echo "public-washing-controller-oracle=PASS safe=PASS unsafe=FAIL unsafe_step=10"
