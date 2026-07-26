#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 COMPILED_MMIO_BUILD OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
build=$(CDPATH= cd -- "$1" && pwd)
output=$2
image="python:3.13-slim@sha256:6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91"

if [ -e "$output" ]; then
  echo "angr baseline output already exists: $output" >&2
  exit 2
fi
mkdir -p "$output"
cleanup() {
  status=$?
  if [ "$status" -ne 0 ]; then
    rm -rf "$output"
  fi
  exit "$status"
}
trap cleanup EXIT HUP INT TERM
output_abs=$(CDPATH= cd -- "$output" && pwd)

docker run --rm \
  -v "$root/scripts/angr-compiled-mmio-baseline-v1.py:/baseline.py:ro" \
  -v "$build:/build:ro" \
  -v "$output_abs:/out" \
  "$image" \
  sh -lc '
    set -eu
    python -m pip install \
      --disable-pip-version-check \
      --no-cache-dir \
      --quiet \
      angr==9.3.0
    python -m pip freeze --all > /out/python-packages.txt
    for profile in o0 o2; do
      python /baseline.py "/build/$profile/firmware.elf" \
        > "/out/$profile-events.txt"
      awk "/^event_count=|^event=|^status=/" \
        "/out/$profile-events.txt" \
        > "/out/$profile-events.normalized.txt"
      awk "/^event_count=|^event=|^status=/" \
        "/build/$profile/native-events.txt" \
        > "/out/$profile-native.normalized.txt"
      cmp \
        "/out/$profile-events.normalized.txt" \
        "/out/$profile-native.normalized.txt"
      rm \
        "/out/$profile-events.normalized.txt" \
        "/out/$profile-native.normalized.txt"
    done
    sha256sum \
      /out/o0-events.txt \
      /out/o2-events.txt \
      /out/python-packages.txt |
      sed "s#  /out/#  #" \
      > /out/manifest.txt
    echo "status=complete" >> /out/manifest.txt
  '

trap - EXIT HUP INT TERM
echo "angr-compiled-mmio-baseline=PASS output=$output_abs"
