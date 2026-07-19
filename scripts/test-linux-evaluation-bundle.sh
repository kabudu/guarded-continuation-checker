#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: $0 SCRATCH_DIRECTORY [RETAINED_OUTPUT_DIRECTORY]" >&2
    exit 2
fi

scratch=$1
retained=${2:-}
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
if [ -e "$scratch" ]; then
    echo "refusing to overwrite scratch directory: $scratch" >&2
    exit 2
fi
mkdir -p "$scratch"
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

if [ -z "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]; then
    git clone --quiet --no-hardlinks "$repo" "$scratch/source-first"
    git clone --quiet --no-hardlinks "$repo" "$scratch/source-second"
    "$scratch/source-first/scripts/build-linux-evaluation-bundle.sh" "$scratch/first"
    "$scratch/source-second/scripts/build-linux-evaluation-bundle.sh" "$scratch/second"
else
    if [ "${GCC_ALLOW_DIRTY_BUNDLE:-0}" != 1 ]; then
        echo "reproducibility test requires a clean source tree" >&2
        exit 2
    fi
    "$repo/scripts/build-linux-evaluation-bundle.sh" "$scratch/first"
    "$repo/scripts/build-linux-evaluation-bundle.sh" "$scratch/second"
fi

first_archive=$(find "$scratch/first" -maxdepth 1 -type f -name '*.tar.gz')
second_archive=$(find "$scratch/second" -maxdepth 1 -type f -name '*.tar.gz')
if [ -z "$first_archive" ] || [ -z "$second_archive" ]; then
    echo "bundle archive is missing" >&2
    exit 1
fi
base=$(basename -- "$first_archive" .tar.gz)
if [ "$(basename -- "$second_archive" .tar.gz)" != "$base" ]; then
    echo "isolated builds produced different archive names" >&2
    exit 1
fi

for suffix in tar.gz tar.gz.sha256 spdx.json intoto.jsonl; do
    cmp "$scratch/first/$base.$suffix" "$scratch/second/$base.$suffix" >/dev/null || {
        echo "isolated builds disagree: $suffix" >&2
        exit 1
    }
done

verify() {
    directory=$1
    "$repo/scripts/verify-linux-evaluation-bundle.sh" \
        "$directory/$base.tar.gz" \
        "$directory/$base.tar.gz.sha256" \
        "$directory/$base.intoto.jsonl" \
        "$directory/$base.spdx.json"
}
rewrite_outer_evidence() {
    directory=$1
    digest=$(sha256sum "$directory/$base.tar.gz" | awk '{print $1}')
    printf '%s  %s.tar.gz\n' "$digest" "$base" >"$directory/$base.tar.gz.sha256"
    jq --arg digest "$digest" '.subject[0].digest.sha256 = $digest' \
        "$directory/$base.intoto.jsonl" >"$directory/provenance.tmp"
    mv "$directory/provenance.tmp" "$directory/$base.intoto.jsonl"
}
verify "$scratch/first"
verify "$scratch/second"

cp -R "$scratch/first" "$scratch/tampered-archive"
printf 'X' | dd of="$scratch/tampered-archive/$base.tar.gz" bs=1 seek=0 conv=notrunc 2>/dev/null
if verify "$scratch/tampered-archive" >/dev/null 2>&1; then
    echo "archive corruption was accepted" >&2
    exit 1
fi

cp -R "$scratch/first" "$scratch/tampered-provenance"
jq '.subject[0].digest.sha256 = "0000000000000000000000000000000000000000000000000000000000000000"' \
    "$scratch/tampered-provenance/$base.intoto.jsonl" \
    >"$scratch/tampered-provenance/provenance.tmp"
mv "$scratch/tampered-provenance/provenance.tmp" \
    "$scratch/tampered-provenance/$base.intoto.jsonl"
if verify "$scratch/tampered-provenance" >/dev/null 2>&1; then
    echo "provenance subject substitution was accepted" >&2
    exit 1
fi

mkdir -p "$scratch/hostile-stage"
tar -xzf "$scratch/first/$base.tar.gz" -C "$scratch/hostile-stage"
cp -R "$scratch/first" "$scratch/hostile-link"
ln -s /tmp "$scratch/hostile-stage/$base/escape"
tar -czf "$scratch/hostile-link/$base.tar.gz" \
    -C "$scratch/hostile-stage" "$base"
rewrite_outer_evidence "$scratch/hostile-link"
if verify "$scratch/hostile-link" >/dev/null 2>&1; then
    echo "archive symlink was accepted" >&2
    exit 1
fi
rm "$scratch/hostile-stage/$base/escape"

cp -R "$scratch/first" "$scratch/hostile-traversal"
tar -czf "$scratch/hostile-traversal/$base.tar.gz" \
    --transform="s|^$base/README.md|$base/../escape|" \
    -C "$scratch/hostile-stage" "$base"
rewrite_outer_evidence "$scratch/hostile-traversal"
if verify "$scratch/hostile-traversal" >/dev/null 2>&1; then
    echo "archive traversal path was accepted" >&2
    exit 1
fi

cp -R "$scratch/first" "$scratch/hostile-executable"
hostile_binary="$scratch/hostile-stage/$base/bin/guarded-continuation-checker"
execution_sentinel="$scratch/candidate-was-executed"
cat >"$hostile_binary" <<'EOF'
#!/bin/sh
touch "$GCC_EXECUTION_SENTINEL"
exit 0
EOF
chmod 0755 "$hostile_binary"
hostile_binary_digest=$(sha256sum "$hostile_binary" | awk '{print $1}')
jq --arg digest "$hostile_binary_digest" '.outputs.binarySha256 = $digest' \
    "$scratch/hostile-stage/$base/BUILD-INFO.json" \
    >"$scratch/hostile-stage/$base/BUILD-INFO.tmp"
mv "$scratch/hostile-stage/$base/BUILD-INFO.tmp" \
    "$scratch/hostile-stage/$base/BUILD-INFO.json"
(
    cd "$scratch/hostile-stage/$base"
    find . -type f ! -name SHA256SUMS -print0 |
        sort -z |
        xargs -0 sha256sum >"$scratch/hostile-SHA256SUMS"
)
mv "$scratch/hostile-SHA256SUMS" \
    "$scratch/hostile-stage/$base/SHA256SUMS"
tar -czf "$scratch/hostile-executable/$base.tar.gz" \
    -C "$scratch/hostile-stage" "$base"
rewrite_outer_evidence "$scratch/hostile-executable"
export GCC_EXECUTION_SENTINEL="$execution_sentinel"
if verify "$scratch/hostile-executable" >/dev/null 2>&1; then
    echo "non-ELF candidate binary was accepted" >&2
    exit 1
fi
unset GCC_EXECUTION_SENTINEL
if [ -e "$execution_sentinel" ]; then
    echo "offline verification executed the candidate binary" >&2
    exit 1
fi

if "$repo/scripts/build-linux-evaluation-bundle.sh" "$scratch/first" >/dev/null 2>&1; then
    echo "bundle builder overwrote an existing output directory" >&2
    exit 1
fi

if [ -n "$retained" ]; then
    if [ -e "$retained" ]; then
        echo "refusing to overwrite retained output directory: $retained" >&2
        exit 2
    fi
    cp -R "$scratch/first" "$retained"
fi

printf 'linux-evaluation-bundle-reproducibility status=PASS archive=%s sha256=%s\n' \
    "$base.tar.gz" "$(sha256sum "$first_archive" | awk '{print $1}')"
