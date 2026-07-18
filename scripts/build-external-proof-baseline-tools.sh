#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

output=$1
cadical_revision=7b99c07f0bcab5824a5a3ce62c7066554017f641
drat_trim_revision=2e5e29cb0019d5cfd547d4208dca1b3ec290349f

if [[ -e "$output" ]]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

mkdir -p "$output/src"
git clone --filter=blob:none https://github.com/arminbiere/cadical.git "$output/src/cadical"
git -C "$output/src/cadical" checkout --detach "$cadical_revision"
(cd "$output/src/cadical" && ./configure)
make -C "$output/src/cadical" -j2

git clone --filter=blob:none https://github.com/marijnheule/drat-trim.git "$output/src/drat-trim"
git -C "$output/src/drat-trim" checkout --detach "$drat_trim_revision"
make -C "$output/src/drat-trim" drat-trim

mkdir -p "$output/bin"
cp "$output/src/cadical/build/cadical" "$output/bin/cadical"
cp "$output/src/drat-trim/drat-trim" "$output/bin/drat-trim"
printf '%s\n' \
  'external_proof_tools_version=1' \
  "cadical_revision=$cadical_revision" \
  "drat_trim_revision=$drat_trim_revision" \
  >"$output/versions.txt"

echo "external proof tools built in $output/bin"
