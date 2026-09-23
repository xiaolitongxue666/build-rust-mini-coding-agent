#!/usr/bin/env bash
# 默认 cargo test（Mock / 无网络）。RUN_LIVE=1 才加载 DeepSeek 密钥（不是 Claude）。
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

if [[ "${RUN_LIVE:-}" == "1" ]]; then
  # shellcheck source=lib/secrets.sh
  source "${SCRIPT_DIR}/lib/secrets.sh"
  load_deepseek_key
  export_live_llm_defaults
  log_info "RUN_LIVE=1：后续测试可以打真实 API"
fi

cargo test
bash "${SCRIPT_DIR}/test-lessons.sh"
