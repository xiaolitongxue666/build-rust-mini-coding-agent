#!/usr/bin/env bash
# 只给 scripts/run.sh 和 RUN_LIVE=1 的 test.sh 用。
# 构建 / 默认单测不要 source 本文件。
#
# 每个 OS / 每个 WSL distro 各自一套凭据，互不翻对方磁盘。
# Windows：MSYS2 / Git Bash 的 $HOME 经常是 /home/<user>，和
# USERPROFILE（C:\Users\...）是同一台 Windows，不是 WSL。
# WSL：只认自己的 $HOME，即使继承了 USERPROFILE 也不读 /mnt/c。
# 代理走宿主机 7890 只是网络出口，不是凭据共用。
#
# 对齐 DeepSeek Harness 凭据层：
# 环境变量 → <home>/.dsh/.credentials.yaml → <home>/.dsh/.env → <home>/.pi/agent/auth.json
# https://deepseekdocs.com/en/docs/user-guide/credentials
# 不读项目根 .env。

# shellcheck source=os.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/os.sh"

_strip_secret() {
  tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^["'\'']//' -e 's/["'\'']$//'
}

# agent-config 的 Pi 模板会把字面量 $DEEPSEEK_API_KEY 写进本环境 auth.json；那不是真密钥。
_usable_key() {
  local value="$1"
  [[ -n "$value" ]] || return 1
  case "$value" in
    '$DEEPSEEK_API_KEY'|'${DEEPSEEK_API_KEY}'|*'$'*)
      return 1
      ;;
  esac
  [[ "$value" == sk-* ]] || return 1
  [[ ${#value} -ge 20 ]] || return 1
  return 0
}

_win_to_posix() {
  local p="$1"
  [[ -n "$p" ]] || return 1
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -u "$p"
    return 0
  fi
  p="${p//\\//}"
  if [[ "$p" =~ ^([A-Za-z]):/(.*)$ ]]; then
    local drive
    drive="$(printf '%s' "${BASH_REMATCH[1]}" | tr 'A-Z' 'a-z')"
    printf '/%s/%s' "$drive" "${BASH_REMATCH[2]}"
    return 0
  fi
  return 1
}

# 仅 detect_os=windows。MSYS `bash` 常清掉 USERPROFILE，不能靠它。
# 不调 cmd.exe / powershell（继承到的也是空环境）。不碰 /mnt/c。
_windows_user_home() {
  [[ "$(detect_os)" == "windows" ]] || return 1
  # run.sh 开了 set -u：local 只声明不赋值就是 unbound。
  local raw="" posix="" name=""
  if [[ -n "${USERPROFILE:-}" ]]; then
    raw="$USERPROFILE"
  elif [[ -n "${HOMEDRIVE:-}" && -n "${HOMEPATH:-}" ]]; then
    raw="${HOMEDRIVE}${HOMEPATH}"
  fi
  if [[ -n "${raw}" ]]; then
    posix="$(_win_to_posix "$raw" || true)"
    if [[ -n "${posix}" && -d "${posix}" ]]; then
      printf '%s' "$posix"
      return 0
    fi
  fi

  name="$(id -un 2>/dev/null || true)"
  [[ -n "${name}" ]] || return 1
  for posix in "/c/Users/${name}" "/cygdrive/c/Users/${name}"; do
    if [[ -d "${posix}" ]]; then
      printf '%s' "$posix"
      return 0
    fi
  done

  posix="$(printf '%s' "${PATH}" | tr ':' '\n' | sed -n "s|^\\(/[a-zA-Z]/Users/${name}\\)/.*|\\1|p" | head -n 1 || true)"
  if [[ -n "${posix}" && -d "${posix}" ]]; then
    printf '%s' "$posix"
    return 0
  fi
  return 1
}

_credential_homes() {
  local homes=()
  local h
  [[ -n "${HOME:-}" ]] && homes+=("$HOME")
  if h="$(_windows_user_home)"; then
    homes+=("$h")
  fi
  local seen=""
  for h in "${homes[@]}"; do
    [[ -n "$h" && -d "$h" ]] || continue
    case " $seen " in
      *" $h "*) continue ;;
    esac
    seen+=" $h"
    printf '%s\n' "$h"
  done
}

_each_existing() {
  local rel="$1"
  local home file
  while IFS= read -r home; do
    file="${home}/${rel}"
    [[ -f "$file" ]] && printf '%s\n' "$file"
  done < <(_credential_homes)
}

# dsh v1：`refs: / DEEPSEEK_API_KEY: sk-...`
_from_dsh_credentials() {
  local file value
  while IFS= read -r file; do
    value="$(
      awk -F': *' '
        /^[[:space:]]*DEEPSEEK_API_KEY:/ {
          val=$2
          for (i = 3; i <= NF; i++) val = val ":" $i
          gsub(/\r/, "", val)
          print val
          exit
        }
      ' "$file" | _strip_secret
    )"
    if _usable_key "$value"; then
      log_info "读取 ${file}"
      printf '%s' "$value"
      return 0
    fi
  done < <(_each_existing ".dsh/.credentials.yaml")
  return 1
}

_from_dsh_env() {
  local file value
  while IFS= read -r file; do
    value="$(
      awk -F= '
        $1 == "DEEPSEEK_API_KEY" {
          sub(/^[^=]*=/, "")
          gsub(/\r/, "")
          print
          exit
        }
      ' "$file" | _strip_secret
    )"
    if _usable_key "$value"; then
      log_info "读取 ${file}"
      printf '%s' "$value"
      return 0
    fi
  done < <(_each_existing ".dsh/.env")
  return 1
}

_from_pi_auth() {
  local file value
  while IFS= read -r file; do
    value="$(awk -F'"' '/"key"[[:space:]]*:/{print $4; exit}' "$file" | _strip_secret)"
    if _usable_key "$value"; then
      log_info "读取 ${file}"
      printf '%s' "$value"
      return 0
    fi
  done < <(_each_existing ".pi/agent/auth.json")
  return 1
}

# 成功时导出 DEEPSEEK_API_KEY。只报来源类别和长度，不打印 key。
load_deepseek_key() {
  if _usable_key "${DEEPSEEK_API_KEY:-}"; then
    log_info "DeepSeek 密钥：环境变量 (len=${#DEEPSEEK_API_KEY})"
    return 0
  fi

  local value
  if value="$(_from_dsh_credentials)"; then
    export DEEPSEEK_API_KEY="$value"
    log_info "DeepSeek 密钥：本环境 dsh credentials (len=${#value})"
    return 0
  fi

  if value="$(_from_dsh_env)"; then
    export DEEPSEEK_API_KEY="$value"
    log_info "DeepSeek 密钥：本环境 dsh .env (len=${#value})"
    return 0
  fi

  if value="$(_from_pi_auth)"; then
    export DEEPSEEK_API_KEY="$value"
    log_info "DeepSeek 密钥：本环境 pi auth.json (len=${#value})"
    return 0
  fi

  log_err "当前环境 HOME=${HOME} 没有可用的 DEEPSEEK_API_KEY（须是 sk- 开头，不能是 \$DEEPSEEK_API_KEY 占位符）。"
  log_err "Windows 认 \$HOME 和 USERPROFILE；WSL 只认自己的 \$HOME，不读 /mnt/c。"
  log_err "在这一侧用 agent-config apply，或 export DEEPSEEK_API_KEY=sk-..."
  return 1
}

export_live_llm_defaults() {
  export LLM_PROVIDER="${LLM_PROVIDER:-deepseek}"
  export LLM_MODEL="${LLM_MODEL:-deepseek-flash}"
  # 官方文档与 dsh adapter 都是 {base}/chat/completions，不要自行再拼 /v1。
  # https://api-docs.deepseek.com/
  export OPENAI_BASE_URL="${OPENAI_BASE_URL:-https://api.deepseek.com}"
}
