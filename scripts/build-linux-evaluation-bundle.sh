#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 OUTPUT_DIRECTORY" >&2
    exit 2
fi

output=$1
target=x86_64-unknown-linux-musl
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)

for command in cargo rustc git jq sha256sum tar gzip find sort xargs date; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "$command is required" >&2
        exit 2
    }
done
tar --version | grep -q 'GNU tar' || {
    echo "GNU tar is required for canonical archive metadata" >&2
    exit 2
}
if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
    echo "the v1 bundle builder requires x86_64 Linux" >&2
    exit 2
fi
if [ -e "$output" ]; then
    echo "refusing to overwrite output directory: $output" >&2
    exit 2
fi
if [ -n "${RUSTFLAGS:-}" ]; then
    echo "RUSTFLAGS must be unset so the canonical build flags cannot be changed" >&2
    exit 2
fi

revision=$(git -C "$repo" rev-parse --verify HEAD)
case "$revision" in
    *[!0-9a-f]*|'') echo "source revision is not canonical" >&2; exit 2 ;;
esac
if [ "${#revision}" -ne 40 ]; then
    echo "source revision is not a full Git object ID" >&2
    exit 2
fi
dirty=false
if [ -n "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]; then
    if [ "${GCC_ALLOW_DIRTY_BUNDLE:-0}" != 1 ]; then
        echo "refusing to package a dirty source tree" >&2
        exit 2
    fi
    dirty=true
fi

source_date_epoch=$(git -C "$repo" show -s --format=%ct "$revision")
if [ -n "${SOURCE_DATE_EPOCH:-}" ] && [ "$SOURCE_DATE_EPOCH" != "$source_date_epoch" ]; then
    echo "SOURCE_DATE_EPOCH must equal the source commit timestamp" >&2
    exit 2
fi
case "$source_date_epoch" in
    *[!0-9]*|'') echo "SOURCE_DATE_EPOCH must be canonical decimal" >&2; exit 2 ;;
esac
created=$(date -u -d "@$source_date_epoch" '+%Y-%m-%dT%H:%M:%SZ')
expected_rust=1.97.0
case "$(rustc --version)" in
    "rustc $expected_rust "*) ;;
    *) echo "Rust $expected_rust is required" >&2; exit 2 ;;
esac
case "$(cargo --version)" in
    "cargo $expected_rust "*) ;;
    *) echo "Cargo $expected_rust is required" >&2; exit 2 ;;
esac
version=$(cargo metadata --manifest-path "$repo/Cargo.toml" --locked --no-deps --format-version 1 |
    jq -er '.packages[] | select(.name == "guarded-continuation-checker") | .version')
case "$version" in
    *[!0-9A-Za-z.+-]*|'') echo "package version is invalid" >&2; exit 2 ;;
esac

base="guarded-continuation-checker-$version-$target"
parent=$(dirname -- "$output")
mkdir -p "$parent"
scratch=$(mktemp -d "$parent/.gcc-linux-bundle.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
stage="$scratch/stage/$base"
result="$scratch/result"
mkdir -p "$stage/bin" "$stage/docs" "$result"

export SOURCE_DATE_EPOCH="$source_date_epoch"
export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$scratch/target"
export LC_ALL=C
export TZ=UTC
export RUSTFLAGS="-C strip=symbols -C link-arg=-Wl,--build-id=none --remap-path-prefix=$repo=/usr/src/guarded-continuation-checker"

cargo build \
    --manifest-path "$repo/Cargo.toml" \
    --release --locked --target "$target"

binary="$CARGO_TARGET_DIR/$target/release/guarded-continuation-checker"
test -x "$binary" || {
    echo "release binary was not produced" >&2
    exit 2
}

cp "$binary" "$stage/bin/guarded-continuation-checker"
cp "$repo/LICENSE" "$stage/LICENSE"
cp "$repo/packaging/linux/README.md" "$stage/README.md"
cp "$repo/docs/OPERATIONS.md" "$stage/docs/OPERATIONS.md"
cp "$repo/docs/ISOLATION_PROFILE_V1.md" "$stage/docs/ISOLATION_PROFILE_V1.md"
cp "$repo/docs/FIRMWARE_CLI_V2.md" "$stage/docs/FIRMWARE_CLI_V2.md"
cp "$repo/docs/PREDICATE_CLI_V1.md" "$stage/docs/PREDICATE_CLI_V1.md"
cp "$repo/docs/EVENT_CONTRACT_CLI_V1.md" "$stage/docs/EVENT_CONTRACT_CLI_V1.md"
cp "$repo/scripts/verify-linux-evaluation-bundle.sh" "$stage/verify-bundle.sh"
chmod 0755 "$stage/bin/guarded-continuation-checker" "$stage/verify-bundle.sh"

{
    "$stage/bin/guarded-continuation-checker" firmware-cli-version
    "$stage/bin/guarded-continuation-checker" predicate-cli-version
    "$stage/bin/guarded-continuation-checker" event-contract-cli-version
} >"$stage/CAPABILITIES.txt"

cargo metadata --manifest-path "$repo/Cargo.toml" --locked --format-version 1 \
    --filter-platform "$target" \
    >"$scratch/cargo-metadata.json"
"$repo/scripts/generate-spdx-sbom.sh" \
    "$scratch/cargo-metadata.json" "$created" "$revision" "$target" \
    "$stage/SBOM.spdx.json"

rustc_version=$(rustc -Vv)
cargo_version=$(cargo -V)
lock_sha256=$(sha256sum "$repo/Cargo.lock" | awk '{print $1}')
binary_sha256=$(sha256sum "$stage/bin/guarded-continuation-checker" | awk '{print $1}')
jq -nS \
    --arg revision "$revision" \
    --argjson dirty "$dirty" \
    --arg sourceDateEpoch "$source_date_epoch" \
    --arg created "$created" \
    --arg target "$target" \
    --arg profile release \
    --arg rustc "$rustc_version" \
    --arg cargo "$cargo_version" \
    --arg cargoLockSha256 "$lock_sha256" \
    --arg binarySha256 "$binary_sha256" '{
      schemaVersion: 1,
      source: {
        repository: "https://github.com/kabudu/guarded-continuation-checker",
        revision: $revision,
        dirty: $dirty
      },
      build: {
        sourceDateEpoch: $sourceDateEpoch,
        created: $created,
        target: $target,
        profile: $profile,
        locked: true,
        rustflags: "-C strip=symbols -C link-arg=-Wl,--build-id=none --remap-path-prefix=SOURCE=/usr/src/guarded-continuation-checker"
      },
      toolchain: { rustc: $rustc, cargo: $cargo },
      materials: { cargoLockSha256: $cargoLockSha256 },
      outputs: { binarySha256: $binarySha256 }
    }' >"$stage/BUILD-INFO.json"

manifest="$scratch/SHA256SUMS"
(
    cd "$stage"
    find . -type f ! -name SHA256SUMS -print0 |
        sort -z |
        xargs -0 sha256sum
) >"$manifest"
mv "$manifest" "$stage/SHA256SUMS"

archive="$result/$base.tar.gz"
tar --sort=name --format=posix \
    --pax-option=delete=atime,delete=ctime \
    --mtime="@$source_date_epoch" --owner=0 --group=0 --numeric-owner \
    --mode='u+rwX,go+rX,go-w' \
    -C "$scratch/stage" -cf - "$base" |
    gzip -n >"$archive"

archive_sha256=$(sha256sum "$archive" | awk '{print $1}')
printf '%s  %s\n' "$archive_sha256" "$base.tar.gz" >"$archive.sha256"
cp "$stage/SBOM.spdx.json" "$result/$base.spdx.json"

jq -cnS \
    --arg name "$base.tar.gz" \
    --arg digest "$archive_sha256" \
    --arg revision "$revision" \
    --arg lockDigest "$lock_sha256" \
    --arg target "$target" \
    --arg sourceDateEpoch "$source_date_epoch" \
    --arg rustc "$rustc_version" \
    --arg cargo "$cargo_version" '{
      _type: "https://in-toto.io/Statement/v1",
      subject: [{name: $name, digest: {sha256: $digest}}],
      predicateType: "https://slsa.dev/provenance/v1",
      predicate: {
        buildDefinition: {
          buildType: "https://guardedcontinuation.org/buildtypes/linux-evaluation-bundle/v1",
          externalParameters: {
            target: $target,
            profile: "release",
            locked: true,
            sourceDateEpoch: $sourceDateEpoch
          },
          internalParameters: {
            rustc: $rustc,
            cargo: $cargo
          },
          resolvedDependencies: [
            {
              uri: "git+https://github.com/kabudu/guarded-continuation-checker",
              digest: {gitCommit: $revision}
            },
            {
              uri: "file:Cargo.lock",
              digest: {sha256: $lockDigest}
            }
          ]
        },
        runDetails: {
          builder: {id: "https://github.com/kabudu/guarded-continuation-checker/scripts/build-linux-evaluation-bundle.sh"},
          metadata: {invocationId: ("local-reproducible:" + $revision)}
        }
      }
    }' >"$result/$base.intoto.jsonl"

mv "$result" "$output"
trap - EXIT HUP INT TERM
rm -rf "$scratch"
printf 'linux-evaluation-bundle status=BUILT archive=%s sha256=%s dirty=%s\n' \
    "$output/$base.tar.gz" "$archive_sha256" "$dirty"
