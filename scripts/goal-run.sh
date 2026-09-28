#!/usr/bin/env bash
# 后学 /goal：maker 一次（现有 harness）→ 验证命令退出码 → 写 loop-state.md。
# 不改 agent_loop。人还在旁边看。
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
echo $$ > "${RUN_DIR}/goal.pid"
cleanup() {
  rm -f "${RUN_DIR}/goal.pid"
}
trap cleanup EXIT

ctl() {
  cargo run --quiet --bin loop-ctl -- "$@"
}

while ctl should-continue; do
  prompt="$(ctl maker-prompt)"
  set +e
  printf '%s\n' "$prompt" | BYO_PLAIN_INPUT=1 BYO_ONCE=1 bash "${SCRIPT_DIR}/run.sh" \
    > "${RUN_DIR}/last-maker.txt" 2>&1
  set -e
  verify="$(ctl verify-command)"
  set +e
  bash -lc "$verify" > "${RUN_DIR}/last-verify.txt" 2>&1
  code=$?
  set -e
  ctl advance --exit-code "$code" \
    --maker-file "${RUN_DIR}/last-maker.txt" \
    --verify-file "${RUN_DIR}/last-verify.txt"
done
