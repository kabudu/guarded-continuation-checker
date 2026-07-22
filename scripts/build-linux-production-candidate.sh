#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 OUTPUT_DIRECTORY" >&2
    exit 2
fi
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
exec "$repo/scripts/build-linux-evaluation-bundle.sh" "$1" firmware-rtl-v1
