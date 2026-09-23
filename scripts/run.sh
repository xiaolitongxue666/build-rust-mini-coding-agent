#!/usr/bin/env bash
# 无参：总体 src/main.rs（cargo run）。有参：单课 example（run.sh 01）。
# 这里才加载 DeepSeek 密钥。Windows MSYS 的 HOME 可能是 /home/<user>。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/rust.sh
source "${SCRIPT_DIR}/lib/rust.sh"
# shellcheck source=lib/secrets.sh
source "${SCRIPT_DIR}/lib/secrets.sh"
# shellcheck source=lib/lessons.sh
source "${SCRIPT_DIR}/lib/lessons.sh"

ROOT="$(repo_root)"
export_proxy_if_available || true
ensure_cargo_path
load_deepseek_key
export_live_llm_defaults
cd "$ROOT"

if [[ $# -eq 0 ]]; then
  cargo run
  exit 0
fi

id="$(normalize_lesson_id "$1")"
row="$(lesson_row "$id")"
if [[ -z "$row" ]]; then
  log_err "没有第 ${id} 课"
  exit 1
fi
status="$(lesson_field "$row" 2)"
how="$(lesson_field "$row" 4)"
if [[ "$status" != "ready" ]]; then
  log_err "第 ${id} 课尚未实现（status=${status}）"
  exit 1
fi
case "$how" in
  cargo_example:*)
    cargo run --example "${how#cargo_example:}"
    ;;
  *)
    log_err "第 ${id} 课没有可运行的 example（how=${how}）"
    exit 1
    ;;
esac
