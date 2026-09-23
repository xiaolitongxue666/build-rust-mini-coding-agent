#!/usr/bin/env bash
# 第 00 课：探测本机 Clash 默认端口 7890。
# 本机用 127.0.0.1；WSL 里 127.0.0.1 是发行版自己，必须改用 Windows 宿主机。
# 这只是网络出口，不是把 Windows / WSL 的家目录或密钥拼在一起。

# shellcheck source=os.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/os.sh"

PROXY_PORT="${PROXY_PORT:-7890}"

wsl_host_ip() {
  local ip
  if [[ -f /etc/resolv.conf ]]; then
    ip="$(awk '/^nameserver/{print $2; exit}' /etc/resolv.conf 2>/dev/null || true)"
    if [[ -n "$ip" ]]; then
      echo "$ip"
      return 0
    fi
  fi
  # resolv.conf 被 systemd-resolved 改写时，退回默认路由网关。
  ip="$(ip route show default 2>/dev/null | awk '{print $3; exit}' || true)"
  if [[ -n "$ip" ]]; then
    echo "$ip"
    return 0
  fi
  return 1
}

proxy_candidate_url() {
  local os
  os="$(detect_os)"
  if [[ "$os" == wsl ]]; then
    local host
    if host="$(wsl_host_ip)"; then
      echo "http://${host}:${PROXY_PORT}"
      return 0
    fi
    log_warn "WSL 未解析到宿主机 IP，回退 127.0.0.1:${PROXY_PORT}"
  fi
  echo "http://127.0.0.1:${PROXY_PORT}"
}

proxy_is_reachable() {
  local url="$1"
  if ! command -v curl >/dev/null 2>&1; then
    return 1
  fi
  # Clash 对 GET / 常回 4xx/5xx；只要 TCP 通、有 HTTP 响应就算探测成功。
  curl -sS -o /dev/null --connect-timeout 1 --max-time 2 "$url" >/dev/null 2>&1
}

# 探测成功才导出。失败不导出，避免 cargo 走一条死代理。
export_proxy_if_available() {
  if [[ -n "${http_proxy:-}${HTTP_PROXY:-}" ]]; then
    log_info "沿用已有 HTTP 代理"
    return 0
  fi

  local url
  url="$(proxy_candidate_url)"
  if proxy_is_reachable "$url"; then
    export http_proxy="$url"
    export https_proxy="$url"
    export HTTP_PROXY="$url"
    export HTTPS_PROXY="$url"
    export ALL_PROXY="$url"
    export all_proxy="$url"
    export NO_PROXY="localhost,127.0.0.1"
    export no_proxy="localhost,127.0.0.1"
    log_info "已启用代理 ${url}"
    return 0
  fi

  log_warn "未探测到 ${url}，本次不设代理"
  return 1
}
