#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 docs/releases/vX.Y.Z.md" >&2
  exit 2
fi

notes=$1
if [[ ! -f "$notes" ]]; then
  echo "release notes do not exist: $notes" >&2
  exit 1
fi

if head -n 1 "$notes" | grep -q '^# '; then
  echo "release notes must not duplicate the GitHub release title" >&2
  exit 1
fi

if ! grep -qx '## Release highlights' "$notes"; then
  echo "release notes require a 'Release highlights' section" >&2
  exit 1
fi

if ! grep -qx '## Status and scope' "$notes"; then
  echo "release notes require a 'Status and scope' section" >&2
  exit 1
fi

em_dash=$'\u2014'
if grep -q "$em_dash" "$notes"; then
  echo "release notes must not contain em dashes" >&2
  exit 1
fi

highlight_count=$(
  awk '
    /^## Release highlights$/ { in_highlights = 1; next }
    /^## / { in_highlights = 0 }
    in_highlights && /^- / { count++ }
    END { print count + 0 }
  ' "$notes"
)
if (( highlight_count < 1 || highlight_count > 7 )); then
  echo "release highlights must contain between 1 and 7 bullets" >&2
  exit 1
fi

if ! awk '
  /^## Release highlights$/ { in_highlights = 1; next }
  /^## / { in_highlights = 0 }
  in_highlights && /^  / { exit 1 }
' "$notes"; then
  echo "release highlight bullets must not continue as prose blocks" >&2
  exit 1
fi

echo "release-notes-format=PASS file=$notes highlights=$highlight_count"
