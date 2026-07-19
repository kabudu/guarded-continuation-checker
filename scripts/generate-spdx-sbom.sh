#!/bin/sh
set -eu

if [ "$#" -ne 5 ]; then
    echo "usage: $0 CARGO-METADATA.json CREATED REVISION TARGET OUTPUT.spdx.json" >&2
    exit 2
fi

metadata=$1
created=$2
revision=$3
target=$4
output=$5

command -v jq >/dev/null 2>&1 || {
    echo "jq is required" >&2
    exit 2
}

case "$created" in
    ????-??-??T??:??:??Z) ;;
    *) echo "CREATED must be canonical UTC RFC 3339 seconds" >&2; exit 2 ;;
esac
case "$revision" in
    *[!0-9a-f]*|'') echo "REVISION must be lowercase hexadecimal" >&2; exit 2 ;;
esac
if [ "${#revision}" -ne 40 ]; then
    echo "REVISION must be a full 40-character Git object ID" >&2
    exit 2
fi
case "$target" in
    *[!A-Za-z0-9_.-]*|'') echo "TARGET is invalid" >&2; exit 2 ;;
esac
if [ -e "$output" ]; then
    echo "refusing to overwrite output: $output" >&2
    exit 2
fi

temporary="$output.tmp.$$"
trap 'rm -f "$temporary"' EXIT HUP INT TERM

jq -S \
    --arg created "$created" \
    --arg revision "$revision" \
    --arg target "$target" '
  def safe:
    gsub("[^A-Za-z0-9.-]"; "-");
  def package_id:
    "SPDXRef-Package-" + (.name | safe) + "-" + (.version | safe);
  def package_ref($packages; $id):
    ($packages | map(select(.id == $id)) | first | package_id);

  . as $metadata
  | ($metadata.packages | sort_by(.name, .version, .id)) as $packages
  | ($packages | map(package_id)) as $package_ids
  | if ($package_ids | unique | length) != ($package_ids | length)
    then error("SPDX package identifiers are not unique")
    else .
    end
  | (package_ref($packages; $metadata.resolve.root)) as $root
  | {
      spdxVersion: "SPDX-2.3",
      dataLicense: "CC0-1.0",
      SPDXID: "SPDXRef-DOCUMENT",
      name: ("guarded-continuation-checker-" + $target),
      documentNamespace: ("https://github.com/kabudu/guarded-continuation-checker/spdx/" + $revision + "/" + $target),
      creationInfo: {
        created: $created,
        creators: [
          "Organization: Guarded Continuation Checker",
          "Tool: cargo-metadata"
        ]
      },
      documentDescribes: [$root],
      packages: ($packages | map({
        SPDXID: package_id,
        name: .name,
        versionInfo: .version,
        downloadLocation: "NOASSERTION",
        filesAnalyzed: false,
        licenseConcluded: "NOASSERTION",
        licenseDeclared: (.license // "NOASSERTION"),
        copyrightText: "NOASSERTION",
        externalRefs: [{
          referenceCategory: "PACKAGE-MANAGER",
          referenceType: "purl",
          referenceLocator: ("pkg:cargo/" + .name + "@" + .version)
        }]
      })),
      relationships: (
        [{
          spdxElementId: "SPDXRef-DOCUMENT",
          relationshipType: "DESCRIBES",
          relatedSpdxElement: $root
        }]
        + [
          $metadata.resolve.nodes[] as $node
          | $node.deps[]
          | {
              spdxElementId: package_ref($packages; $node.id),
              relationshipType: "DEPENDS_ON",
              relatedSpdxElement: package_ref($packages; .pkg)
            }
        ]
        | unique_by(.spdxElementId, .relationshipType, .relatedSpdxElement)
        | sort_by(.spdxElementId, .relationshipType, .relatedSpdxElement)
      )
    }
' "$metadata" >"$temporary"

jq -e '
  . as $document
  | ($document.packages | map(.SPDXID)) as $ids
  | .spdxVersion == "SPDX-2.3"
  and .dataLicense == "CC0-1.0"
  and (.packages | length > 1)
  and (.documentDescribes | length == 1)
  and ([.packages[].SPDXID] | length == (unique | length))
  and ([.relationships[].spdxElementId, .relationships[].relatedSpdxElement]
       | all(. == "SPDXRef-DOCUMENT" or (. as $id | $ids | index($id) != null)))
' "$temporary" >/dev/null

mv "$temporary" "$output"
trap - EXIT HUP INT TERM
