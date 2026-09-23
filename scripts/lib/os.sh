#!/usr/bin/env bash
# 第 00 课：识别运行环境。后续脚本按这个值选代理地址和安装方式。
# 返回值约定：windows | macos | linux | wsl
# windows 与 wsl 兼容但完全隔离：密钥和家目录互不借用。

detect_os() {
  local uname_s
  uname_s="$(uname -s 2>/dev/null || echo unknown)"

  # WSL 必须先于 generic linux：内核字符串和 WSL_DISTRO_NAME 都是可靠信号。
  if [[ -n "${WSL_DISTRO_NAME:-}" ]] \
    || [[ -f /proc/sys/fs/binfmt_misc/WSLInterop ]] \
    || grep -qiE 'microsoft|wsl' /proc/version 2>/dev/null; then
    echo wsl
    return 0
  fi

  case "$uname_s" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      echo windows
      ;;
    Darwin)
      echo macos
      ;;
    Linux)
      echo linux
      ;;
    *)
      echo linux
      ;;
  esac
}

repo_root() {
  # 无论从哪一层 scripts/*.sh 调用，都回到仓库根。
  local here
  here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  echo "$here"
}

log_info() { printf '[INFO] %s\n' "$*" >&2; }
log_warn() { printf '[WARN] %s\n' "$*" >&2; }
log_err()  { printf '[ERR ] %s\n' "$*" >&2; }
