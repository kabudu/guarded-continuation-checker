#!/usr/bin/env bash
set -euo pipefail

repository=$(cd "$(dirname "$0")/.." && pwd -P)
root=$repository/corpus/rtl/wmcontroller
manifest=$root/composed-witness-plants-v1/source-model-provenance-v1.txt
expected=$repository/results/composed-witness-plants-source-model-attestation-v1.csv
attester=$repository/scripts/attest-source-model-provenance.sh
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-changing-plants.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

"$attester" "$root" "$manifest" "$scratch/actual.csv" >/dev/null
diff -u "$expected" "$scratch/actual.csv"

expect_failure() {
  local expected_exit=$1 expected_message=$2
  shift 2
  set +e
  message=$("$@" 2>&1)
  actual_exit=$?
  set -e
  [[ $actual_exit -eq $expected_exit ]] || {
    echo "expected exit $expected_exit, got $actual_exit" >&2
    exit 1
  }
  grep -qx "$expected_message" <<<"$message"
}

cp -R "$root" "$scratch/drift"
sed -i.bak "s/assign sig_lid_closed = 1'b1;/assign sig_lid_closed = 1'b0;/" \
  "$scratch/drift/composed-witness-plants-v1/sensor-stuck/physical-plant.v"
rm "$scratch/drift/composed-witness-plants-v1/sensor-stuck/physical-plant.v.bak"
expect_failure 1 'source-to-model regeneration mismatch for member 1' \
  "$attester" "$scratch/drift" \
  "$scratch/drift/composed-witness-plants-v1/source-model-provenance-v1.txt" \
  "$scratch/drift.csv"

cp -R "$root" "$scratch/model-drift"
printf 'mutation\n' >> \
  "$scratch/model-drift/composed-witness-plants-v1/actuator-delay/physical-plant.aag"
expect_failure 1 'source-to-model regeneration mismatch for member 2' \
  "$attester" "$scratch/model-drift" \
  "$scratch/model-drift/composed-witness-plants-v1/source-model-provenance-v1.txt" \
  "$scratch/model-drift.csv"

sed 's|workdir=composed-witness-plants-v1/persistent-disturbance|workdir=composed-witness-plants-v1/sensor-stuck|' \
  "$manifest" >"$scratch/substitution.txt"
expect_failure 2 'provenance manifest contains duplicate member subject' \
  "$attester" "$root" "$scratch/substitution.txt" \
  "$scratch/substitution.csv"

echo 'changing-plant provenance tests status=PASS attested=4 hostile=3'
