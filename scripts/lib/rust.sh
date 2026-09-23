#!/usr/bin/env bash
# 第 00 课：确保 rustup / rustc / cargo / rustfmt / clippy 可用。
# 不预装其它语言运行时。安装走官方 rustup；若已导出代理则 rustup 会跟环境变量走。

# shellcheck source=os.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/os.sh"

ensure_cargo_path() {
  if [[ -f "${HOME}/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "${HOME}/.cargo/env"
  fi
  # 只认当前 $HOME。WSL 不读 /mnt/c 的 rustup；MSYS 的 HOME 可能是 /home/<user>，
  # 那时靠 PATH 里已有的本机 cargo，不要去翻另一套 OS。
  if [[ -d "${HOME}/.cargo/bin" ]]; then
    case ":$PATH:" in
      *":${HOME}/.cargo/bin:"*) ;;
      *) export PATH="${HOME}/.cargo/bin:${PATH}" ;;
    esac
  fi
}

install_rustup() {
  if command -v rustup >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    return 0
  fi

  log_info "未找到 rustup/cargo，开始安装 stable toolchain"
  if ! command -v curl >/dev/null 2>&1; then
    log_err "需要 curl 才能安装 rustup"
    return 1
  fi

  # -y：非交互。默认 host triple 由 rustup 自己判断（含 GNU/MSVC）。
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  ensure_cargo_path
}

ensure_rust_components() {
  ensure_cargo_path
  if ! command -v rustup >/dev/null 2>&1; then
    log_err "rustup 仍不可用"
    return 1
  fi
  rustup component add rustfmt clippy >/dev/null
  log_info "toolchain: $(rustc --version 2>/dev/null || echo missing)"
}

ensure_rust() {
  install_rustup
  ensure_rust_components
}
