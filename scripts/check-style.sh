#!/usr/bin/env bash
set -euo pipefail

repository=$(cd "$(dirname "$0")/.." && pwd)
forbidden=$(printf '\342\200\224')

if rg -n "$forbidden" "$repository" \
  --hidden --glob '!.git/**' --glob '!target/**'; then
  echo "style check failed: replace em dashes with context-appropriate punctuation" >&2
  exit 1
fi

echo "style-check=PASS"
