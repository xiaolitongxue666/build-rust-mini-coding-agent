#!/usr/bin/env bash
# 启动第 01 课快照。这里才加载 DeepSeek 密钥（不是 Claude）。
# Windows MSYS 的 HOME 可能是 /home/<user>；secrets.sh 会再认本机用户目录。
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

ROOT="$(repo_root)"
export_proxy_if_available || true
ensure_cargo_path
load_deepseek_key
export_live_llm_defaults
cd "$ROOT"
# 第 01 课的可运行快照。第 02 课起改成 cargo run（src/ 母本）。
cargo run --example lesson_01
