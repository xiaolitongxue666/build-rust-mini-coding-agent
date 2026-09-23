#!/usr/bin/env bash
# 完整测试流程：离线 check →（可选）管道实机门/读写 → 键位手测清单。
# 默认不读 API key。只有 --live 才加载当前环境 DeepSeek 密钥。
#
#   bash scripts/test-flow.sh          # Phase A + C
#   bash scripts/test-flow.sh --live   # A + B + C（打真实模型）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/rust.sh
source "${SCRIPT_DIR}/lib/rust.sh"

ROOT="$(repo_root)"
LIVE=0
for arg in "$@"; do
  case "$arg" in
    --live) LIVE=1 ;;
    -h|--help)
      sed -n '2,8p' "$0"
      exit 0
      ;;
    *)
      log_err "未知参数: $arg"
      exit 2
      ;;
  esac
done

export_proxy_if_available || true
ensure_cargo_path
cd "$ROOT"

PROBE_DIR="${ROOT}/.local/live-probe"
NOTE="${PROBE_DIR}/note.txt"
LOG="${PROBE_DIR}/agent.log"
AGENT_PID=""

cleanup_probe() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
}

print_key_checklist() {
  cat <<'EOF'

[Phase C] 键位手测（管道测不到整屏 TUI）。入口：bash scripts/run.sh
  - 备用屏：启动盖住原来的终端；Ctrl+D 退回原样
  - 上面是可翻视口，下面是圆角框 + ❯
  - 等模型时输入框上有转圈；委托时应出现 · research
  - PgUp 停自动跟随；End 再跟到底
  - 审批是黄框，按一下 y/n，不用回车
  - Ctrl+C 一次清空输入框；连续两次或 Ctrl+D 无报错退出
  - ↑↓ 翻历史；↓ 越过最新一条应回到按 ↑ 前的草稿
  - 重启后仍能翻到上次提交（$HOME/.rustbyo_harness_history）
EOF
}

phase_a_offline() {
  log_info "Phase A：离线 check.sh（不读密钥）"
  bash "${SCRIPT_DIR}/check.sh"
}

agent_fs_path() {
  local f="$1"
  if [[ "$(detect_os)" == windows ]]; then
    if command -v cygpath >/dev/null 2>&1; then
      cygpath -w "$f"
      return
    fi
    local dir
    dir="$(cd "$(dirname "$f")" && pwd -W 2>/dev/null || pwd)"
    printf '%s/%s\n' "$dir" "$(basename "$f")"
    return
  fi
  printf '%s\n' "$f"
}

run_agent_script() {
  local secs="$1"
  local script="$2"
  export BYO_PLAIN_INPUT=1
  set +e
  printf '%s' "$script" | cargo run >"$LOG" 2>&1 &
  AGENT_PID=$!
  local i=0
  while kill -0 "${AGENT_PID}" 2>/dev/null; do
    if [[ "$i" -ge "$secs" ]]; then
      log_err "Phase B：agent 超过 ${secs}s，杀掉"
      kill "${AGENT_PID}" 2>/dev/null || true
      wait "${AGENT_PID}" 2>/dev/null || true
      AGENT_PID=""
      set -e
      return 124
    fi
    sleep 1
    i=$((i + 1))
  done
  wait "${AGENT_PID}"
  local rc=$?
  AGENT_PID=""
  set -e
  return "$rc"
}

assert_file() {
  local want="$1"
  local got
  got="$(tr -d '\r' <"$NOTE" | sed -e 's/[[:space:]]*$//')"
  if [[ "$got" != "$want" ]]; then
    log_err "Phase B：${NOTE} 期望 ${want}，实际：${got}"
    return 1
  fi
}

phase_b_live() {
  # shellcheck source=lib/secrets.sh
  source "${SCRIPT_DIR}/lib/secrets.sh"
  load_deepseek_key
  export_live_llm_defaults

  log_info "Phase B：probe-llm（密钥/网络；失败不是 agent 逻辑）"
  if ! bash "${SCRIPT_DIR}/probe-llm.sh"; then
    log_err "Phase B：probe 失败，停止 live（检查当前环境密钥与代理）"
    return 1
  fi

  mkdir -p "$PROBE_DIR"
  printf '%s\n' "LIVE_PROBE_V1" >"$NOTE"
  local path
  path="$(agent_fs_path "$NOTE")"
  log_info "Phase B：管道启动总体 agent  path=${path}"

  local script
  script="$(printf '%s\n' \
    "这一轮只调用一次 read_file，不要调用其它工具，不要凭记忆编造。路径：${path}" \
    "y" \
    "这一轮只调用一次 write_file，把同一个文件改成只含一行 LIVE_PROBE_V2，不要写别的字。路径：${path}" \
    "y" \
    "这一轮只调用一次 write_file，把同一个文件改成只含一行 SHOULD_NOT。路径：${path}" \
    "n")"

  trap cleanup_probe EXIT
  if ! run_agent_script 180 "$script"; then
    local rc=$?
    log_err "Phase B：cargo run 退出码 ${rc}。日志：${LOG}"
    return 1
  fi
  trap - EXIT

  if ! grep -q '\[tool\] read_file' "$LOG"; then
    log_err "Phase B：日志没有 [tool] read_file（模型没用工具）。${LOG}"
    return 1
  fi
  write_n="$(grep -c '\[tool\] write_file' "$LOG" || true)"
  if [[ "$write_n" -lt 2 ]]; then
    log_err "Phase B：需要两次 write_file（批准一次、拒绝一次），实际 ${write_n}。${LOG}"
    return 1
  fi
  approve_n="$(grep -c 'approve?' "$LOG" || true)"
  if [[ "$approve_n" -lt 3 ]]; then
    log_err "Phase B：需要至少 3 次 approve?，实际 ${approve_n}。${LOG}"
    return 1
  fi
  assert_file "LIVE_PROBE_V2"
  log_info "Phase B：读 y / 写 y / 写 n 均通过，文件仍是 LIVE_PROBE_V2"
  rm -rf "$PROBE_DIR"
}

phase_a_offline
if [[ "$LIVE" -eq 1 ]]; then
  phase_b_live
fi
print_key_checklist
log_info "test-flow 完成（live=${LIVE}）"
