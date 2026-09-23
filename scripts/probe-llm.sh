#!/usr/bin/env bash
# 打一次 DeepSeek 官方 Chat Completions，确认密钥和路径。不打印 key。
# 课上的 Claude /v1 Messages 不是这条线；不要自行再拼 /v1。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/secrets.sh
source "${SCRIPT_DIR}/lib/secrets.sh"

export_proxy_if_available || true
load_deepseek_key
export_live_llm_defaults

url="${OPENAI_BASE_URL%/}/chat/completions"
body='{"model":"'"${LLM_MODEL}"'","messages":[{"role":"user","content":"ping"}],"max_tokens":8}'
tmp="$(mktemp)"
hdr="$(mktemp)"
cleanup() { rm -f "$tmp" "$hdr"; }
trap cleanup EXIT

curl -sS -D "$hdr" -o "$tmp" --connect-timeout 8 --max-time 30 \
  "$url" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${DEEPSEEK_API_KEY}" \
  -d "$body"

status="$(awk 'BEGIN{s="?"} /^HTTP\//{s=$2} END{print s}' "$hdr")"
log_info "probe ${url} model=${LLM_MODEL} http=${status}"
if [[ "$status" != "200" ]]; then
  python -c "
import json,sys
raw=open(r'''$tmp''',encoding='utf-8',errors='replace').read()
try:
    d=json.loads(raw)
    err=d.get('error') or {}
    print('type=', err.get('type'), ' code=', err.get('code'), sep='')
except Exception:
    print('body_len', len(raw))
" >&2
  exit 1
fi
