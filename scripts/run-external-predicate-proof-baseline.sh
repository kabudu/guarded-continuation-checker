#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 OBLIGATION_BUNDLE CADICAL DRAT_TRIM OUTPUT.csv" >&2
  exit 2
fi

bundle=$1
cadical=$2
drat_trim=$3
output=$4

if [[ ! -d "$bundle" || ! -f "$bundle/manifest.txt" ]]; then
  echo "obligation bundle or manifest is missing" >&2
  exit 2
fi
if [[ ! -x "$cadical" || ! -x "$drat_trim" ]]; then
  echo "external proof tools must be executable regular files" >&2
  exit 2
fi
if [[ -e "$output" ]]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-external-proof.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT

cadical_sha256=$(shasum -a 256 "$cadical" | awk '{print $1}')
drat_trim_sha256=$(shasum -a 256 "$drat_trim" | awk '{print $1}')
manifest_sha256=$(shasum -a 256 "$bundle/manifest.txt" | awk '{print $1}')
temporary="$scratch/results.csv"
printf '%s\n' 'schema_version,manifest_sha256,cadical_sha256,drat_trim_sha256,obligation,kind,phase,source,cnf_bytes,proof_bytes,produce_ns,check_ns,producer_status,checker_status,status' >"$temporary"

while IFS= read -r line; do
  [[ "$line" =~ ^obligation_[0-9]+= ]] || continue
  value=${line#*=}
  IFS=, read -r filename kind phase source expected_sha256 <<<"$value"
  if [[ ! "$filename" =~ ^[a-z0-9-]+\.cnf$ ]] \
    || [[ ! "$kind" =~ ^(relation-completeness|terminal-completeness)$ ]] \
    || [[ ! "$phase" =~ ^(-|[0-9]+)$ ]] \
    || [[ ! "$source" =~ ^(-|[0-9]+)$ ]] \
    || [[ ! "$expected_sha256" =~ ^[0-9a-f]{64}$ ]]; then
    echo "invalid obligation manifest row: $line" >&2
    exit 2
  fi
  cnf="$bundle/$filename"
  if [[ ! -f "$cnf" || -L "$cnf" ]]; then
    echo "obligation is not a regular file: $filename" >&2
    exit 2
  fi
  actual_sha256=$(shasum -a 256 "$cnf" | awk '{print $1}')
  if [[ "$actual_sha256" != "$expected_sha256" ]]; then
    echo "obligation digest mismatch: $filename" >&2
    exit 2
  fi

  proof="$scratch/${filename%.cnf}.drat"
  produce_start=$(date +%s%N)
  set +e
  (
    ulimit -f 1048576
    ulimit -v 2097152
    # DRAT-trim v05.22.2023 misreads some CaDiCaL 3.0 binary traces, while the
    # semantically identical canonical text DRAT stream verifies reliably.
    timeout 300s "$cadical" --no-binary "$cnf" "$proof"
  ) >"$scratch/cadical.log" 2>&1
  producer_status=$?
  set -e
  produce_ns=$(( $(date +%s%N) - produce_start ))
  if [[ $producer_status -ne 20 ]]; then
    echo "CaDiCaL did not prove UNSAT for $filename (status $producer_status)" >&2
    exit 1
  fi

  check_start=$(date +%s%N)
  set +e
  (
    ulimit -f 1048576
    ulimit -v 2097152
    timeout 300s "$drat_trim" "$cnf" "$proof"
  ) >"$scratch/drat-trim.log" 2>&1
  checker_status=$?
  set -e
  check_ns=$(( $(date +%s%N) - check_start ))
  if [[ $checker_status -ne 0 ]] || ! grep -q 's VERIFIED' "$scratch/drat-trim.log"; then
    echo "DRAT-trim rejected $filename (status $checker_status)" >&2
    exit 1
  fi

  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
    "$manifest_sha256" "$cadical_sha256" "$drat_trim_sha256" \
    "$filename" "$kind" "$phase" "$source" \
    "$(wc -c <"$cnf" | tr -d ' ')" "$(wc -c <"$proof" | tr -d ' ')" \
    "$produce_ns" "$check_ns" "$producer_status" "$checker_status" \
    >>"$temporary"
done <"$bundle/manifest.txt"

expected=$(sed -n 's/^obligation_count=//p' "$bundle/manifest.txt")
actual=$(( $(wc -l <"$temporary") - 1 ))
if [[ ! "$expected" =~ ^[0-9]+$ || "$actual" -ne "$expected" ]]; then
  echo "obligation count mismatch: expected $expected, checked $actual" >&2
  exit 1
fi

aggregate=$(sed -n 's/^aggregate_cnf=//p' "$bundle/manifest.txt")
aggregate_sha256=$(sed -n 's/^aggregate_sha256=//p' "$bundle/manifest.txt")
if [[ "$aggregate" != "aggregate.cnf" ]] || [[ ! "$aggregate_sha256" =~ ^[0-9a-f]{64}$ ]]; then
  echo "aggregate obligation metadata is invalid" >&2
  exit 2
fi
aggregate_cnf="$bundle/$aggregate"
if [[ ! -f "$aggregate_cnf" || -L "$aggregate_cnf" ]] \
  || [[ "$(shasum -a 256 "$aggregate_cnf" | awk '{print $1}')" != "$aggregate_sha256" ]]; then
  echo "aggregate obligation is missing, linked, or changed" >&2
  exit 2
fi
aggregate_proof="$scratch/aggregate.drat"
produce_start=$(date +%s%N)
set +e
(
  ulimit -f 1048576
  ulimit -v 2097152
  timeout 300s "$cadical" --no-binary "$aggregate_cnf" "$aggregate_proof"
) >"$scratch/cadical-aggregate.log" 2>&1
producer_status=$?
set -e
produce_ns=$(( $(date +%s%N) - produce_start ))
if [[ $producer_status -ne 20 ]]; then
  echo "CaDiCaL did not prove the aggregate UNSAT (status $producer_status)" >&2
  exit 1
fi
check_start=$(date +%s%N)
set +e
(
  ulimit -f 1048576
  ulimit -v 2097152
  timeout 300s "$drat_trim" "$aggregate_cnf" "$aggregate_proof"
) >"$scratch/drat-trim-aggregate.log" 2>&1
checker_status=$?
set -e
check_ns=$(( $(date +%s%N) - check_start ))
if [[ $checker_status -ne 0 ]] || ! grep -q 's VERIFIED' "$scratch/drat-trim-aggregate.log"; then
  echo "DRAT-trim rejected the aggregate (status $checker_status)" >&2
  exit 1
fi
printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
  "$manifest_sha256" "$cadical_sha256" "$drat_trim_sha256" \
  "$aggregate" 'aggregate-completeness' '-' '-' \
  "$(wc -c <"$aggregate_cnf" | tr -d ' ')" \
  "$(wc -c <"$aggregate_proof" | tr -d ' ')" \
  "$produce_ns" "$check_ns" "$producer_status" "$checker_status" \
  >>"$temporary"

mv "$temporary" "$output"
echo "external predicate proof baseline status=VALID obligations=$actual aggregate=VALID output=$output"
