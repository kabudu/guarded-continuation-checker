#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 SOURCE_ROOT MANIFEST.txt OUTPUT.csv" >&2
  exit 2
fi

root=$1
manifest=$2
output=$3
[[ -d "$root" && ! -L "$root" ]] || { echo "source root must be a non-symlink directory" >&2; exit 2; }
[[ -f "$manifest" && ! -L "$manifest" ]] || { echo "provenance manifest must be a non-symlink file" >&2; exit 2; }
[[ ! -e "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
[[ $(wc -c <"$manifest") -le 65536 ]] || { echo "provenance manifest exceeds 65536 bytes" >&2; exit 2; }
if LC_ALL=C grep -q $'\r' "$manifest" || ! LC_ALL=C tr -d '\000' <"$manifest" | cmp -s - "$manifest"; then
  echo "provenance manifest contains prohibited bytes" >&2
  exit 2
fi
[[ $(tail -c 1 "$manifest" | wc -l) -eq 1 ]] || { echo "provenance manifest must end with LF" >&2; exit 2; }

root=$(cd "$root" && pwd -P)
manifest=$(cd "$(dirname "$manifest")" && pwd -P)/$(basename "$manifest")
mkdir -p "$(dirname "$output")"
output=$(cd "$(dirname "$output")" && pwd -P)/$(basename "$output")
[[ -z $(find "$root" -type l -print -quit) ]] || { echo "source tree must not contain symlinks" >&2; exit 2; }
tree_files=$(find "$root" -type f -print | wc -l | tr -d ' ')
tree_kib=$(du -sk "$root" | awk '{print $1}')
[[ "$tree_files" =~ ^[0-9]+$ && "$tree_files" -le 4096 ]] || {
  echo "source tree file count exceeds 4096" >&2; exit 2;
}
[[ "$tree_kib" =~ ^[0-9]+$ && "$tree_kib" -le 262144 ]] || {
  echo "source tree exceeds 256 MiB" >&2; exit 2;
}

exec 3<"$manifest"
take() {
  local expected=$1 line
  IFS= read -r line <&3 || { echo "provenance manifest expected $expected" >&2; exit 2; }
  [[ "$line" == "$expected="* ]] || { echo "provenance manifest expected $expected" >&2; exit 2; }
  REPLY=${line#*=}
  [[ -n "$REPLY" ]] || { echo "provenance manifest $expected is empty" >&2; exit 2; }
}

take source_model_provenance_manifest_version
[[ "$REPLY" == 1 ]] || { echo "unsupported provenance manifest version" >&2; exit 2; }
take tool
[[ "$REPLY" == yosys ]] || { echo "unsupported provenance tool" >&2; exit 2; }
take tool_revision
tool_revision=$REPLY
[[ "$tool_revision" =~ ^[0-9a-f]{40}$ ]] || { echo "provenance tool revision is invalid" >&2; exit 2; }
take member_count
member_count=$REPLY
[[ "$member_count" =~ ^[1-9][0-9]*$ && "$member_count" -le 64 ]] || {
  echo "provenance member count is outside limits" >&2; exit 2;
}

declare -a workdirs sources recipes models
validate_path() {
  local value=$1 allow_dot=$2 segment
  [[ -n "$value" && "$value" != /* && "$value" != *//* ]] || return 1
  [[ "$allow_dot" == yes && "$value" == . ]] && return 0
  IFS=/ read -ra segments <<<"$value"
  for segment in "${segments[@]}"; do
    [[ -n "$segment" && "$segment" != . && "$segment" != .. ]] || return 1
    [[ "$segment" =~ ^[A-Za-z0-9._-]+$ ]] || return 1
  done
}
for ((index = 0; index < member_count; index++)); do
  take workdir; workdirs[index]=$REPLY
  take source_path; sources[index]=$REPLY
  take recipe_path; recipes[index]=$REPLY
  take model_path; models[index]=$REPLY
  validate_path "${workdirs[index]}" yes || { echo "provenance workdir path is invalid" >&2; exit 2; }
  validate_path "${sources[index]}" no || { echo "provenance source path is invalid" >&2; exit 2; }
  validate_path "${recipes[index]}" no || { echo "provenance recipe path is invalid" >&2; exit 2; }
  validate_path "${models[index]}" no || { echo "provenance model path is invalid" >&2; exit 2; }
done
take status
[[ "$REPLY" == complete ]] || { echo "provenance manifest status is incomplete" >&2; exit 2; }
if IFS= read -r _ <&3; then
  echo "provenance manifest has trailing fields" >&2
  exit 2
fi
exec 3<&-

command -v yosys >/dev/null 2>&1 || { echo "pinned Yosys is required" >&2; exit 2; }
yosys_version=$(yosys -V)
[[ "$yosys_version" == *"git sha1 $tool_revision,"* ]] || {
  echo "Yosys revision does not match provenance manifest" >&2
  exit 2
}
if command -v timeout >/dev/null 2>&1; then
  deadline=(timeout 120)
elif command -v perl >/dev/null 2>&1; then
  deadline=(perl -e '$seconds = shift; $SIG{ALRM} = sub { exit 124 }; alarm $seconds; exec @ARGV' 120)
else
  echo "timeout or perl is required for the synthesis deadline" >&2
  exit 2
fi

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

assert_regular_path() {
  local base=$1 relative=$2 current component
  current=$base
  IFS=/ read -ra components <<<"$relative"
  for component in "${components[@]}"; do
    current=$current/$component
    [[ ! -L "$current" ]] || { echo "provenance path contains symlink: $relative" >&2; exit 2; }
  done
  [[ -f "$current" ]] || { echo "provenance file is missing: $relative" >&2; exit 2; }
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-source-model.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
cp -R "$root/." "$scratch/tree"
result=$scratch/attestation.csv
printf 'schema_version,member,tool,tool_revision,source_sha256,recipe_sha256,model_sha256,regenerated_sha256,byte_match,status\n' >"$result"

for ((index = 0; index < member_count; index++)); do
  workdir=${workdirs[index]}
  [[ "$workdir" == . ]] && workdir=
  member_root=$root${workdir:+/$workdir}
  copied_root=$scratch/tree${workdir:+/$workdir}
  [[ -d "$member_root" && ! -L "$member_root" ]] || { echo "provenance workdir is invalid" >&2; exit 2; }
  assert_regular_path "$member_root" "${sources[index]}"
  assert_regular_path "$member_root" "${recipes[index]}"
  assert_regular_path "$member_root" "${models[index]}"
  source_sha256=$(sha256_file "$member_root/${sources[index]}")
  recipe_sha256=$(sha256_file "$member_root/${recipes[index]}")
  model_sha256=$(sha256_file "$member_root/${models[index]}")
  if command -v prlimit >/dev/null 2>&1; then
    (cd "$copied_root" && prlimit --as=1073741824 --fsize=67108864 --nproc=64 -- \
      "${deadline[@]}" yosys --no-version -Q -q -s "${recipes[index]}")
  else
    (cd "$copied_root" && "${deadline[@]}" yosys --no-version -Q -q -s "${recipes[index]}")
  fi
  regenerated_sha256=$(sha256_file "$copied_root/${models[index]}")
  [[ "$regenerated_sha256" == "$model_sha256" ]] || {
    echo "source-to-model regeneration mismatch for member $index" >&2
    exit 1
  }
  printf '1,%s,yosys,%s,%s,%s,%s,%s,true,attested\n' \
    "$index" "$tool_revision" "$source_sha256" "$recipe_sha256" \
    "$model_sha256" "$regenerated_sha256" >>"$result"
done

mv "$result" "$output"
echo "source-model provenance status=ATTESTED members=$member_count tool=yosys revision=$tool_revision output=$output"
