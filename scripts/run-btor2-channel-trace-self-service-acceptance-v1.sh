#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
fixture=$repository/corpus/rtl/opentitan-pwm-channel-family
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-channel-trace-acceptance.XXXXXXXX")
working_output=$(mktemp "$output.tmp.XXXXXXXX")
cleanup() {
  rm -rf "$scratch"
  if [ -n "$working_output" ]; then
    rm -f "$working_output"
  fi
}
trap cleanup EXIT HUP INT TERM

cp "$fixture/generated/symbolic-class-6.btor2" "$scratch/model.btor2"
cp "$fixture/trace-queries-v1.txt" "$scratch/queries.txt"
cp "$fixture/trace-policy-v1.txt" "$scratch/policy.txt"
model=$scratch/model.btor2
queries=$scratch/queries.txt
policy=$scratch/policy.txt
artifact=$scratch/result.channel-traces

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "a SHA-256 utility is required" >&2
    exit 2
  fi
}

record() {
  printf '1,%s,%s,%s,%s,%s,%s\n' "$1" "$2" "$3" "$4" "$5" "$6" >>"$working_output"
}

printf '%s\n' \
  'schema_version,case,expected_exit,actual_exit,artifact_present,artifact_sha256,status' \
  >"$working_output"

"$binary" btor2-channel-trace-cli-version >"$scratch/capability.stdout"
grep -q '^btor2_channel_trace_cli_version=1 ' "$scratch/capability.stdout"
record capability-discovery 0 0 false none accepted

"$binary" certify-btor2-channel-traces \
  "$model" "$queries" "$policy" "$artifact" >"$scratch/certify.stdout"
test "$(grep -c '^btor2-channel-trace index=' "$scratch/certify.stdout")" -eq 42
test "$(grep -c ' answer=SAFE ' "$scratch/certify.stdout")" -eq 6
test "$(grep -c ' answer=UNSAFE ' "$scratch/certify.stdout")" -eq 36
artifact_sha256=$(sha256_file "$artifact")
test "$artifact_sha256" = 9ca8d6bdb0ee10877a29711fbb518810b28908b64b831283db5e2db3688ecf4a
record full-cohort-certification 0 0 true "$artifact_sha256" accepted

"$binary" verify-btor2-channel-traces \
  "$model" "$queries" "$policy" "$artifact" >"$scratch/verify.stdout"
test "$(grep -c '^btor2-channel-trace index=' "$scratch/verify.stdout")" -eq 42
record independent-verification 0 0 true "$artifact_sha256" accepted

if "$binary" certify-btor2-channel-traces \
  "$model" "$queries" "$policy" "$artifact" \
  >"$scratch/collision.stdout" 2>"$scratch/collision.stderr"; then
  echo "existing-output case unexpectedly succeeded" >&2
  exit 1
else
  status=$?
fi
test "$status" -eq 2
test "$(sha256_file "$artifact")" = "$artifact_sha256"
record existing-output-refusal 2 "$status" true "$artifact_sha256" accepted

sed 's/query=0,0,1,1,1,1/query=0,0,1,1,0,1/' "$queries" >"$scratch/drifted-queries.txt"
if "$binary" verify-btor2-channel-traces \
  "$model" "$scratch/drifted-queries.txt" "$policy" "$artifact" \
  >"$scratch/drift.stdout" 2>"$scratch/drift.stderr"; then
  echo "query-drift case unexpectedly succeeded" >&2
  exit 1
else
  status=$?
fi
test "$status" -eq 2
record query-drift-refusal 2 "$status" true "$artifact_sha256" accepted

sed 's/max_projected_work=100000000000/max_projected_work=1/' \
  "$policy" >"$scratch/tight-policy.txt"
refused_artifact=$scratch/refused.channel-traces
if "$binary" certify-btor2-channel-traces \
  "$model" "$queries" "$scratch/tight-policy.txt" "$refused_artifact" \
  >"$scratch/refusal.stdout" 2>"$scratch/refusal.stderr"; then
  echo "resource-refusal case unexpectedly succeeded" >&2
  exit 1
else
  status=$?
fi
test "$status" -eq 3
test ! -e "$refused_artifact"
test "$(cat "$scratch/refusal.stderr")" = \
  'error: btor2-channel-trace-resource refusal=projected-work result=none'
record resource-refusal 3 "$status" false none accepted

ln "$working_output" "$output"
rm -f "$working_output"
working_output=
echo "btor2_channel_trace_self_service_acceptance_v1=ACCEPTED cases=6 output=$output"
