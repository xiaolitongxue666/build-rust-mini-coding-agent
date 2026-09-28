#!/usr/bin/env bash
# 后学 /graph：implement → verify → 路由。pause 时退出等人 /graph approve。
# 不改 agent_loop。不自动 git commit。
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

THREAD="${GRAPH_THREAD:-session-1}"
export GRAPH_THREAD="$THREAD"
RUN_DIR=".local/graph/${THREAD}"
mkdir -p "$RUN_DIR"
echo $$ > "${RUN_DIR}/graph.pid"
cleanup() {
  rm -f "${RUN_DIR}/graph.pid"
}
trap cleanup EXIT

ctl() {
  cargo run --quiet --bin loop-ctl -- "$@"
}

while ctl graph-should-continue; do
  current="$(ctl graph-current)"
  case "$current" in
    implement)
      prompt="$(ctl graph-maker-prompt)"
      set +e
      printf '%s\n' "$prompt" | BYO_PLAIN_INPUT=1 BYO_ONCE=1 bash "${SCRIPT_DIR}/run.sh" \
        > "${RUN_DIR}/last-implement.txt" 2>&1
      set -e
      ctl graph-step implement --notes-file "${RUN_DIR}/last-implement.txt"
      ;;
    verify)
      verify="$(ctl verify-command)"
      set +e
      bash -lc "$verify" > "${RUN_DIR}/last-verify.txt" 2>&1
      code=$?
      set -e
      ctl graph-step verify --exit-code "$code" --verify-file "${RUN_DIR}/last-verify.txt"
      ;;
    merge)
      ctl graph-step merge
      ;;
    *)
      break
      ;;
  esac
done
