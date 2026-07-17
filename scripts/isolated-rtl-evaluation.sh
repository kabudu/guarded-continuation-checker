#!/usr/bin/env bash
set -euo pipefail

readonly IMAGE='hdlc/yosys@sha256:58c0c80e41fd96b4b90da53c730aa3c43051f0cf2a6c6e336bd012281479df22'
readonly PROFILE_VERSION=1
readonly MEMORY_BYTES=3221225472
readonly PIDS_LIMIT=128
readonly CPU_LIMIT=2
readonly EVALUATION_TIMEOUT_SECONDS=300
readonly AUXILIARY_TIMEOUT_SECONDS=30

if [[ $# -ne 3 ]]; then
  echo "usage: $0 CQ_LINUX_BINARY PROJECT_CONFIG OUTPUT_DIR" >&2
  exit 2
fi

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 2; }
[[ -f "$1" && -x "$1" ]] || { echo "CQ Linux binary is missing or not executable: $1" >&2; exit 2; }
[[ -f "$2" ]] || { echo "project config is missing: $2" >&2; exit 2; }
[[ ! -L "$2" ]] || { echo "project config may not be a symlink: $2" >&2; exit 2; }
[[ ! -e "$3" ]] || { echo "output directory must not already exist: $3" >&2; exit 2; }
[[ ! -e "$3.isolation-report.txt" ]] || {
  echo "isolation report must not already exist: $3.isolation-report.txt" >&2
  exit 2
}

binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
config=$(cd "$(dirname "$2")" && pwd -P)/$(basename "$2")
input_root=$(dirname "$config")
config_name=$(basename "$config")
mkdir -p "$(dirname "$3")"
output_parent=$(cd "$(dirname "$3")" && pwd -P)
output="$output_parent/$(basename "$3")"
report="$output.isolation-report.txt"

for path in "$binary" "$config" "$input_root" "$output"; do
  case "$path" in
    *:*|*$'\n'*|*$'\r'*)
      echo "Docker bind paths may not contain colons or line breaks: $path" >&2
      exit 2
      ;;
  esac
done
[[ "$input_root" != / ]] || { echo "refusing to mount the host root as input" >&2; exit 2; }
case "$output/" in
  "$input_root/"*)
    echo "output directory may not be inside the read-only input tree" >&2
    exit 2
    ;;
esac

docker_os=$(docker info --format '{{.OSType}}')
[[ "$docker_os" == linux ]] || { echo "Docker must use a Linux daemon" >&2; exit 2; }
docker_security=$(docker info --format '{{json .SecurityOptions}}')
[[ "$docker_security" == *'name=seccomp'* ]] || {
  echo "Docker daemon does not report seccomp enforcement" >&2
  exit 2
}
docker image inspect "$IMAGE" >/dev/null 2>&1 || {
  echo "pinned isolation image is not installed; pull it during trusted provisioning: $IMAGE" >&2
  exit 2
}

readonly runtime_user="$(id -u):$(id -g)"
common=(
  --rm --pull never --platform linux/amd64
  --network none --read-only --cap-drop ALL
  --security-opt no-new-privileges=true
  --pids-limit "$PIDS_LIMIT"
  --memory "$MEMORY_BYTES" --memory-swap "$MEMORY_BYTES"
  --cpus "$CPU_LIMIT" --ulimit nofile=1024:1024
  --user "$runtime_user"
  --tmpfs /tmp:rw,noexec,nosuid,nodev,size=67108864,mode=1777
  --volume "$binary:/opt/cq:ro"
)

probe="$output_parent/.cq-isolation-probe-$$"
active_cid_file=
container_sequence=0
umask 077
mkdir "$probe"
cleanup() {
  if [[ -n "$active_cid_file" && -s "$active_cid_file" ]]; then
    docker kill "$(cat "$active_cid_file")" >/dev/null 2>&1 || true
  fi
  [[ -z "$active_cid_file" ]] || rm -f -- "$active_cid_file"
  rm -rf -- "$probe"
}
on_signal() {
  trap - EXIT INT TERM
  cleanup
  exit 2
}
trap cleanup EXIT
trap on_signal INT TERM

run_container() {
  local timeout_seconds=$1
  shift
  container_sequence=$((container_sequence + 1))
  active_cid_file="$output_parent/.cq-isolation-cid-$$-$container_sequence"
  docker run --cidfile "$active_cid_file" "$@" &
  local client_pid=$!
  local deadline=$((SECONDS + timeout_seconds))
  while kill -0 "$client_pid" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      if [[ -s "$active_cid_file" ]]; then
        docker kill "$(cat "$active_cid_file")" >/dev/null 2>&1 || true
      fi
      wait "$client_pid" >/dev/null 2>&1 || true
      rm -f -- "$active_cid_file"
      active_cid_file=
      return 124
    fi
    sleep 1
  done
  local code=0
  wait "$client_pid" || code=$?
  rm -f -- "$active_cid_file"
  active_cid_file=
  return "$code"
}

if ! run_container "$AUXILIARY_TIMEOUT_SECONDS" "${common[@]}" \
  --volume "$input_root:/input:ro" \
  --volume "$probe:/output:rw" \
  "$IMAGE" sh -eu -c '
    test "$(/opt/cq firmware-cli-version)" = "firmware_cli_version=2 artifact_schema_version=4"
    test "$(id -u)" != 0
    test "$(awk "/^CapEff:/ { print \$2 }" /proc/self/status)" = 0000000000000000
    test "$(awk "/^NoNewPrivs:/ { print \$2 }" /proc/self/status)" = 1
    test "$(awk "/^Seccomp:/ { print \$2 }" /proc/self/status)" = 2
    test "$(cat /sys/fs/cgroup/memory.max)" = 3221225472
    test "$(cat /sys/fs/cgroup/memory.swap.max)" = 0
    test "$(cat /sys/fs/cgroup/pids.max)" = 128
    test "$(cat /sys/fs/cgroup/cpu.max)" = "200000 100000"
    test "$(wc -l < /proc/net/route)" = 1
    test "$(awk "{ if (\$10 != \"lo\") bad=1 } END { print bad+0 }" /proc/net/ipv6_route)" = 0
    ! touch /cq-rootfs-write-probe 2>/dev/null
    ! touch /input/cq-input-write-probe 2>/dev/null
    touch /output/cq-output-write-probe
    rm /output/cq-output-write-probe
  '; then
  echo "container runtime failed the hostile-RTL isolation probe" >&2
  exit 2
fi

if [[ "${CQ_ISOLATION_WATCHDOG_SELF_TEST:-0}" == 1 ]]; then
  set +e
  run_container 1 "${common[@]}" "$IMAGE" sh -c 'sleep 60'
  watchdog_exit=$?
  set -e
  if [[ $watchdog_exit -ne 124 ]]; then
    echo "container watchdog self-test returned $watchdog_exit instead of 124" >&2
    exit 2
  fi
  echo 'isolation-watchdog-self-test=PASS'
  exit 2
fi

mkdir "$output"
set +e
run_container "$EVALUATION_TIMEOUT_SECONDS" "${common[@]}" \
  --volume "$input_root:/input:ro" \
  --volume "$output:/output:rw" \
  "$IMAGE" /opt/cq firmware-rtl-config-safety-gate "/input/$config_name" /output
gate_exit=$?
set -e
if [[ $gate_exit -ne 0 && $gate_exit -ne 1 ]]; then
  echo "isolated safety gate failed with exit $gate_exit; evidence retained at $output" >&2
  exit 2
fi

if ! run_container "$AUXILIARY_TIMEOUT_SECONDS" "${common[@]}" \
  --volume "$output:/output:ro" \
  "$IMAGE" /opt/cq firmware-artifact-validate /output; then
  echo "isolated evidence validation failed or exceeded its deadline" >&2
  exit 2
fi

status=$(sed -n 's/^status=//p' "$output/run-manifest.txt" | head -n 1)
if [[ ($gate_exit -eq 0 && "$status" != SAFE) || ($gate_exit -eq 1 && "$status" != UNSAFE) ]]; then
  echo "isolated gate exit and validated status disagree" >&2
  exit 2
fi

temporary_report=$(mktemp "$report.tmp.XXXXXX")
printf '%s\n' \
  "isolation_profile_version=$PROFILE_VERSION" \
  "image=$IMAGE" \
  'platform=linux/amd64' \
  "runtime_user=$runtime_user" \
  'network=none' \
  'rootfs_read_only=true' \
  'capabilities=none' \
  'no_new_privileges=true' \
  'seccomp=filtering' \
  "pids_limit=$PIDS_LIMIT" \
  "memory_limit_bytes=$MEMORY_BYTES" \
  "memory_plus_swap_limit_bytes=$MEMORY_BYTES" \
  'swap_limit_bytes=0' \
  "cpu_limit=$CPU_LIMIT" \
  "evaluation_timeout_seconds=$EVALUATION_TIMEOUT_SECONDS" \
  "auxiliary_timeout_seconds=$AUXILIARY_TIMEOUT_SECONDS" \
  'input_mount=read-only' \
  'output_mount=dedicated' \
  'runtime_probe=PASS' \
  "result=$status" > "$temporary_report"
mv "$temporary_report" "$report"

echo "isolated-rtl-evaluation status=$status evidence=$output isolation_report=$report"
exit "$gate_exit"
