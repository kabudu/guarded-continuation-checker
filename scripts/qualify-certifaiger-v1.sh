#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 CHECKOUT_ROOT OUTPUT_DIR" >&2
  echo "CHECKOUT_ROOT must contain certifaiger, aiger, cadical, lrat_isa, and runlim." >&2
  exit 2
}

[[ $# -eq 2 ]] || usage
checkout_root=$(cd "$1" && pwd)
output_dir=$2
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
lock="$repo_root/tools/certifaiger-qualification-v1.lock"

[[ ! -e "$output_dir" ]] || {
  echo "refusing to overwrite output: $output_dir" >&2
  exit 2
}
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)

lock_value() {
  local key=$1
  local value
  value=$(sed -n "s/^${key}=//p" "$lock")
  [[ -n "$value" ]] || {
    echo "missing lock value: $key" >&2
    exit 2
  }
  printf '%s' "$value"
}

verify_checkout() {
  local name=$1
  local expected
  local actual
  expected=$(lock_value "${name}_commit")
  [[ -d "$checkout_root/$name/.git" ]] || {
    echo "missing checkout: $checkout_root/$name" >&2
    exit 2
  }
  actual=$(git -C "$checkout_root/$name" rev-parse HEAD)
  [[ "$actual" == "$expected" ]] || {
    echo "$name revision mismatch: expected $expected, found $actual" >&2
    exit 2
  }
  [[ -z "$(git -C "$checkout_root/$name" status --porcelain)" ]] || {
    echo "$name checkout is dirty" >&2
    exit 2
  }
}

for checkout in certifaiger aiger cadical lrat_isa runlim; do
  verify_checkout "$checkout"
done

qualification_image=$(lock_value qualification_image)
cp "$lock" "$output_dir/revisions.lock"
sha256sum "$output_dir/revisions.lock" > "$output_dir/revisions.lock.sha256"

docker run --rm \
  --network none \
  --volume "$checkout_root:/src:ro" \
  --volume "$output_dir:/out" \
  "$qualification_image" \
  bash -euo pipefail -c '
    command -v cmake >/dev/null
    command -v g++ >/dev/null
    command -v make >/dev/null
    cp -a /src/certifaiger /tmp/certifaiger
    cp -a /src/aiger /tmp/aiger
    cp -a /src/cadical /tmp/cadical
    cp -a /src/lrat_isa /tmp/lrat_isa
    cp -a /src/runlim /tmp/runlim
    cmake -S /tmp/certifaiger -B /tmp/certifaiger/build \
      -DCMAKE_BUILD_TYPE=Release \
      -DCHECK=ON \
      -DSAT=cadical \
      -DPROOF=lrat_isa \
      -DSTATIC=OFF \
      -DAIGER_DIR=/tmp/aiger \
      -DCADICAL_DIR=/tmp/cadical \
      -DLRAT_ISA_DIR=/tmp/lrat_isa \
      -DRUNLIM_DIR=/tmp/runlim
    cmake --build /tmp/certifaiger/build --parallel "$(nproc)"
    cmake --install /tmp/certifaiger/build --prefix /tmp/install
    /tmp/install/bin/certifaiger --version
    test_log=/out/upstream-tests.log
    : > "$test_log"
    while IFS= read -r witness; do
      [[ -n "$witness" ]] || continue
      model=${witness/witness/model}
      expected=valid
      if grep -Fxq "$witness" /tmp/certifaiger/tests/expected-invalid; then
        expected=invalid
      fi
      if /tmp/install/bin/check \
          "/tmp/certifaiger/tests/$model" \
          "/tmp/certifaiger/tests/$witness" \
          >> "$test_log" 2>&1; then
        actual=valid
      else
        actual=invalid
      fi
      printf "%s expected=%s actual=%s\n" "$witness" "$expected" "$actual" \
        >> "$test_log"
      [[ "$actual" == "$expected" ]]
    done < <(find /tmp/certifaiger/tests -maxdepth 1 -type f \
      -name "*_witness.*" -printf "%f\n" | sort)
    cp -a /tmp/install/bin /out/bin
  ' > "$output_dir/build.log" 2>&1

find "$output_dir/bin" -type f -exec sha256sum {} + | sort > "$output_dir/binaries.sha256"
printf 'qualification=pass\n' > "$output_dir/result.txt"
