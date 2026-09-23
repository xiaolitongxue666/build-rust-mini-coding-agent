#!/usr/bin/env bash
# 第 00 课：课号登记表。test-lessons.sh / run.sh 只认这里。
#
# 单课 demo = examples/NN-*.rs。总体 = src/main.rs。
# 新课：加 example + Cargo.toml 一行 + 这里改 ready / cargo_example:lesson_NN，再并进库。
# 课上的例子走 Claude；本仓库 ready 课的 live 路径是 DeepSeek。
#
# status:
#   ready   — 已实现，脚本会跑
#   pending — 还没写，list 能看见，点名运行则非 0
#   docs    — 只有 markdown，没有可测代码
# how:
#   cargo_example:<name>  → cargo test --example <name>
#   cargo_test:<filter>   → cargo test <filter>
#   （空）                  → 无 cargo 目标

lesson_catalog() {
  cat <<'EOF'
00|docs|Introduction|
01|ready|The agent loop|cargo_example:lesson_01
02|ready|The permission gate|cargo_example:lesson_02
03|ready|The provider interface|cargo_example:lesson_03
04|ready|UI polish|cargo_example:lesson_04
05|pending|Slash commands|
06|pending|Conversation state|
07|pending|Compaction strategies|
08|pending|Better input|
09|pending|Plug-and-play tools|
10|pending|Project structure|
11|pending|Subagents|
12|pending|The full TUI|
13|docs|What's next|
14|pending|Adding MCP support|
15|pending|Project context with AGENTS.md|
16|pending|The token viewer|
17|pending|Prompt caching|
18|pending|Diff approval for writes|
19|pending|Agent memory|
EOF
}

normalize_lesson_id() {
  local raw="${1:-}"
  raw="${raw#lesson_}"
  raw="${raw#lesson-}"
  printf '%02d' "$((10#$raw))"
}

lesson_row() {
  local id="$1"
  lesson_catalog | awk -F'|' -v id="$id" '$1==id {print; exit}'
}

lesson_field() {
  local row="$1"
  local idx="$2"
  printf '%s' "$row" | awk -F'|' -v i="$idx" '{print $i}'
}
