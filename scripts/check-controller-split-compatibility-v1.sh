#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 GCC_BINARY" >&2
  exit 2
fi

test -x "$1"
binary=$(CDPATH='' cd -- "$(dirname "$1")" && pwd)/$(basename "$1")
repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-split-compatibility.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
actual=$scratch/controller-split-resource-acceptance-v1.csv
expected=$repository/results/controller-split-resource-acceptance-v1.csv

(
  cd "$repository"
  scripts/run-controller-split-resource-acceptance.sh "$binary" "$actual" >/dev/null
)

if ! diff -u "$expected" "$actual"; then
  echo "controller split v1 compatibility baseline changed" >&2
  exit 1
fi

echo "controller-split-compatibility-v1=PASS baseline=controller-split-resource-acceptance-v1"
