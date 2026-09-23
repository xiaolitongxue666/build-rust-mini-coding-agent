#!/usr/bin/env bash
# fmt + clippy + test。不读 API key。
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

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
bash "${SCRIPT_DIR}/test.sh"
bash "${SCRIPT_DIR}/test-lessons.sh"
