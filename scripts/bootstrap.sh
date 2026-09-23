#!/usr/bin/env bash
# 安装 rustup / rustc / rustfmt / clippy。不读 API key。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/rust.sh
source "${SCRIPT_DIR}/lib/rust.sh"

log_info "os=$(detect_os)"
export_proxy_if_available || true
ensure_rust
log_info "bootstrap 完成"
