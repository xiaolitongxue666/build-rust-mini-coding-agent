#!/usr/bin/env bash
# 按课号跑测试。默认只跑 status=ready 的课，不读 API key。
#
#   bash scripts/test-lessons.sh           # 全部已实现
#   bash scripts/test-lessons.sh 01        # 只跑第 01 课
#   bash scripts/test-lessons.sh list      # 打印登记表
#
# 第 02 课及之后实现时：在 scripts/lib/lessons.sh 把 pending 改成 ready，
# 并写上 cargo_test:lesson_0N 或新的 example 名。不要为了测试去复制整份 agent。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/os.sh
source "${SCRIPT_DIR}/lib/os.sh"
# shellcheck source=lib/proxy.sh
source "${SCRIPT_DIR}/lib/proxy.sh"
# shellcheck source=lib/rust.sh
source "${SCRIPT_DIR}/lib/rust.sh"
# shellcheck source=lib/lessons.sh
source "${SCRIPT_DIR}/lib/lessons.sh"

ROOT="$(repo_root)"
export_proxy_if_available || true
ensure_cargo_path
cd "$ROOT"

print_catalog() {
  printf '%-4s %-8s %s\n' "ID" "STATUS" "TITLE"
  while IFS='|' read -r id status title _how; do
    printf '%-4s %-8s %s\n' "$id" "$status" "$title"
  done < <(lesson_catalog)
}

run_how() {
  local how="$1"
  case "$how" in
    cargo_example:*)
      cargo test --example "${how#cargo_example:}"
      ;;
    cargo_test:*)
      cargo test "${how#cargo_test:}"
      ;;
    "")
      log_info "无 cargo 目标，跳过"
      ;;
    *)
      log_err "未知 how: $how"
      return 1
      ;;
  esac
}

run_one() {
  local id="$1"
  local row status title how
  row="$(lesson_row "$id")"
  if [[ -z "$row" ]]; then
    log_err "没有第 ${id} 课"
    return 1
  fi
  status="$(lesson_field "$row" 2)"
  title="$(lesson_field "$row" 3)"
  how="$(lesson_field "$row" 4)"

  case "$status" in
    ready)
      log_info "lesson ${id} ${title}"
      run_how "$how"
      ;;
    docs)
      log_info "lesson ${id} ${title}（只有文档）"
      ;;
    pending)
      log_warn "lesson ${id} ${title} 尚未实现"
      return 2
      ;;
    *)
      log_err "lesson ${id} 状态异常: $status"
      return 1
      ;;
  esac
}

ARG="${1:-all}"

if [[ "$ARG" == "list" || "$ARG" == "-l" ]]; then
  print_catalog
  exit 0
fi

if [[ "$ARG" == "all" ]]; then
  failed=0
  while IFS='|' read -r id status _title _how; do
    [[ "$status" == ready ]] || continue
    if ! run_one "$id"; then
      failed=1
    fi
  done < <(lesson_catalog)
  if [[ "$failed" -ne 0 ]]; then
    exit 1
  fi
  exit 0
fi

run_one "$(normalize_lesson_id "$ARG")"
