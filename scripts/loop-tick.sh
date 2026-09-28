#!/usr/bin/env bash
# 后学 /loop：按间隔新起进程跑同一句巡检。不读 loop-state.md。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/rust.sh
source "${SCRIPT_DIR}/lib/rust.sh"

ROOT="$(repo_root)"
export_proxy_if_available || true
ensure_cargo_path
cd "$ROOT"

RUN_DIR=".local/loop"
mkdir -p "$RUN_DIR"
echo $$ > "${RUN_DIR}/patrol.pid"
cleanup() {
  rm -f "${RUN_DIR}/patrol.pid"
}
trap cleanup EXIT

ctl() {
  cargo run --quiet --bin loop-ctl -- "$@"
}

secs="$(ctl interval-seconds)"
prompt="$(ctl patrol-prompt)"

while [[ ! -f "${RUN_DIR}/patrol.stop" ]]; do
  printf '%s\n' "$prompt" | BYO_PLAIN_INPUT=1 BYO_ONCE=1 bash "${SCRIPT_DIR}/run.sh" \
    >> "${RUN_DIR}/patrol.log" 2>&1 || true
  left="$secs"
  while (( left > 0 )) && [[ ! -f "${RUN_DIR}/patrol.stop" ]]; do
    chunk=5
    if (( left < chunk )); then
      chunk="$left"
    fi
    sleep "$chunk"
    left=$((left - chunk))
  done
done
