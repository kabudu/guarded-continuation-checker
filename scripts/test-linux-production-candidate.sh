#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: $0 SCRATCH_DIRECTORY [RETAINED_OUTPUT_DIRECTORY]" >&2
    exit 2
fi
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
exec "$repo/scripts/test-linux-evaluation-bundle.sh" "$1" "${2:-}" firmware-rtl-v1
