#!/bin/sh
set -eu

if [ "$#" -ne 4 ]; then
    echo "usage: $0 ARCHIVE.tar.gz ARCHIVE.tar.gz.sha256 PROVENANCE.intoto.jsonl SBOM.spdx.json" >&2
    exit 2
fi

archive=$1
checksum=$2
provenance=$3
external_sbom=$4
export LC_ALL=C
export TZ=UTC

for command in jq sha256sum tar gzip sort uniq cmp readelf stat awk grep cat wc; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "$command is required" >&2
        exit 2
    }
done
for file in "$archive" "$checksum" "$provenance" "$external_sbom"; do
    if [ ! -f "$file" ] || [ -L "$file" ]; then
        echo "bundle input must be a regular non-symlink file: $file" >&2
        exit 2
    fi
done
if [ "$(stat -c '%s' "$archive")" -gt 134217728 ]; then
    echo "archive exceeds the 128 MiB verification limit" >&2
    exit 2
fi
if [ "$(stat -c '%s' "$checksum")" -gt 256 ] ||
   [ "$(stat -c '%s' "$provenance")" -gt 1048576 ] ||
   [ "$(stat -c '%s' "$external_sbom")" -gt 8388608 ]; then
    echo "bundle metadata exceeds verification limits" >&2
    exit 2
fi

archive_name=$(basename -- "$archive")
case "$archive_name" in
    guarded-continuation-checker-*-x86_64-unknown-linux-musl.tar.gz) ;;
    *) echo "archive name is not canonical" >&2; exit 2 ;;
esac
root=${archive_name%.tar.gz}

if ! awk -v name="$archive_name" '
    NR != 1 || NF != 2 || length($1) != 64 || $1 !~ /^[0-9a-f]+$/ || $2 != name { exit 1 }
    END { if (NR != 1) exit 1 }
' "$checksum"; then
    echo "archive checksum file is not canonical" >&2
    exit 2
fi
expected_digest=$(awk '{print $1}' "$checksum")
actual_digest=$(sha256sum "$archive" | awk '{print $1}')
if [ "$actual_digest" != "$expected_digest" ]; then
    echo "archive checksum mismatch" >&2
    exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-bundle-verify.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
entries="$scratch/entries.txt"
tar --list --gzip --file "$archive" --quoting-style=escape >"$entries"
entry_count=$(awk 'END {print NR}' "$entries")
if [ "$entry_count" -lt 10 ] || [ "$entry_count" -gt 128 ]; then
    echo "archive entry count is out of bounds" >&2
    exit 2
fi
if sort "$entries" | uniq -d | grep -q .; then
    echo "archive contains duplicate entries" >&2
    exit 2
fi
if ! awk -v root="$root" '
    $0 == "" || $0 ~ /[^A-Za-z0-9._\/-]/ ||
    $0 ~ /(^|\/)\.\.?(\/|$)/ || $0 ~ /\/\// { exit 1 }
    $0 != root && index($0, root "/") != 1 { exit 1 }
    END { if (NR == 0) exit 1 }
' "$entries"; then
    echo "archive contains a non-canonical path" >&2
    exit 2
fi
if ! tar -tvzf "$archive" | awk '$1 !~ /^[d-]/ { exit 1 }'; then
    echo "archive contains a link or special file" >&2
    exit 2
fi

# Limit each extracted file to 64 MiB even if a hostile archive lies about size.
ulimit -f 131072
tar --extract --gzip --file "$archive" --directory "$scratch" \
    --no-same-owner --no-same-permissions
bundle="$scratch/$root"
for required in \
    SHA256SUMS BUILD-INFO.json CAPABILITIES.txt SBOM.spdx.json LICENSE README.md \
    bin/guarded-continuation-checker verify-bundle.sh; do
    if [ ! -f "$bundle/$required" ] || [ -L "$bundle/$required" ]; then
        echo "bundle is missing required regular file: $required" >&2
        exit 2
    fi
done

(
    cd "$bundle"
    sha256sum --strict -c SHA256SUMS >/dev/null
)
cmp "$external_sbom" "$bundle/SBOM.spdx.json" >/dev/null || {
    echo "external and embedded SBOMs disagree" >&2
    exit 2
}

jq -e '
  . as $document
  | ($document.packages | map(.SPDXID)) as $ids
  | .spdxVersion == "SPDX-2.3"
  and .dataLicense == "CC0-1.0"
  and .SPDXID == "SPDXRef-DOCUMENT"
  and (.documentNamespace | startswith("https://github.com/kabudu/guarded-continuation-checker/spdx/"))
  and (.creationInfo.created | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"))
  and (.packages | length > 1)
  and (.documentDescribes | length == 1)
  and ($ids | length == (unique | length))
  and ([.relationships[].spdxElementId, .relationships[].relatedSpdxElement]
       | all(. == "SPDXRef-DOCUMENT" or (. as $id | $ids | index($id) != null)))
' "$external_sbom" >/dev/null || {
    echo "SPDX document failed structural verification" >&2
    exit 2
}

if [ "$(wc -l < "$provenance" | tr -d ' ')" -ne 1 ]; then
    echo "provenance must contain one newline-terminated JSON statement" >&2
    exit 2
fi
jq -e \
    --arg name "$archive_name" \
    --arg digest "$actual_digest" '
  ._type == "https://in-toto.io/Statement/v1"
  and .predicateType == "https://slsa.dev/provenance/v1"
  and (.subject | length == 1)
  and .subject[0].name == $name
  and .subject[0].digest.sha256 == $digest
  and .predicate.buildDefinition.buildType ==
      "https://guardedcontinuation.org/buildtypes/linux-evaluation-bundle/v1"
  and .predicate.buildDefinition.externalParameters.target ==
      "x86_64-unknown-linux-musl"
  and .predicate.buildDefinition.externalParameters.locked == true
  and (.predicate.buildDefinition.resolvedDependencies | length == 2)
' "$provenance" >/dev/null || {
    echo "provenance statement failed structural or subject verification" >&2
    exit 2
}

dirty=$(jq -r '
  .source.dirty
  | if . == true then "true" elif . == false then "false" else error("invalid dirty flag") end
' "$bundle/BUILD-INFO.json")
if [ "$dirty" != false ] && [ "${GCC_ALLOW_DIRTY_BUNDLE:-0}" != 1 ]; then
    echo "bundle was built from a dirty source tree" >&2
    exit 2
fi
revision=$(jq -er '.source.revision' "$bundle/BUILD-INFO.json")
lock_digest=$(jq -er '.materials.cargoLockSha256' "$bundle/BUILD-INFO.json")
binary_digest=$(jq -er '.outputs.binarySha256' "$bundle/BUILD-INFO.json")
case "$revision:$lock_digest:$binary_digest" in
    *[!0-9a-f:]*) echo "build information contains invalid digests" >&2; exit 2 ;;
esac
if [ "${#revision}" -ne 40 ] || [ "${#lock_digest}" -ne 64 ] ||
   [ "${#binary_digest}" -ne 64 ]; then
    echo "build information digest lengths are invalid" >&2
    exit 2
fi
if [ "$(sha256sum "$bundle/bin/guarded-continuation-checker" | awk '{print $1}')" != "$binary_digest" ]; then
    echo "binary digest disagrees with build information" >&2
    exit 2
fi
if ! jq -e --arg revision "$revision" --arg lock "$lock_digest" '
  any(.predicate.buildDefinition.resolvedDependencies[];
      .digest.gitCommit == $revision)
  and any(.predicate.buildDefinition.resolvedDependencies[];
          .digest.sha256 == $lock)
' "$provenance" >/dev/null; then
    echo "build information and provenance materials disagree" >&2
    exit 2
fi

elf_header="$scratch/elf-header.txt"
elf_program_headers="$scratch/elf-program-headers.txt"
if ! readelf -h "$bundle/bin/guarded-continuation-checker" >"$elf_header" 2>/dev/null ||
   ! readelf -l "$bundle/bin/guarded-continuation-checker" >"$elf_program_headers" 2>/dev/null; then
    echo "evaluation binary is not a valid ELF executable" >&2
    exit 2
fi
if ! grep -Eq '^[[:space:]]*Class:[[:space:]]+ELF64$' "$elf_header" ||
   ! grep -Eq "^[[:space:]]*Data:[[:space:]]+2's complement, little endian$" "$elf_header" ||
   ! grep -Eq '^[[:space:]]*Machine:[[:space:]]+Advanced Micro Devices X86-64$' "$elf_header"; then
    echo "evaluation binary architecture disagrees with the bundle target" >&2
    exit 2
fi
if grep -q 'Requesting program interpreter' "$elf_program_headers"; then
    echo "evaluation binary is dynamically linked" >&2
    exit 2
fi

# Do not execute the candidate binary here. Offline verification establishes
# integrity and structure, not publisher identity. Execution belongs after
# signature verification and inside the documented isolation boundary.

printf 'linux-evaluation-bundle status=VERIFIED archive=%s sha256=%s revision=%s\n' \
    "$archive_name" "$actual_digest" "$revision"
