# 08 · 更好的输入

对照：[Better input](https://www.byoharness.dev/chapters/08-better-input.html) · 本课代码：[`examples/08-better-input.rs`](../../examples/08-better-input.rs)

`stdin.lines()` 读一行。不能在行中移动光标、翻历史、改三个字之前的笔误。对话 REPL 里消息往往很长，这样很难用。

分两步改，路程比终点有用。

## 步骤 1：readline

Unix 经典答案是 readline。本仓库第 04 课用 **rustyline**：方向键、Backspace、Ctrl-A/E、↑↓ 历史、Ctrl-D 退出、Ctrl-C 清行。这是 80/20。

课上还有 `HistoryFile`。第 04 课故意不写盘（密钥会进历史文件）。本课接上：写当前 `$HOME/.rustbyo_harness_history`，不写另一侧 OS 的家目录。

## 步骤 2：一次性边框输入

readline 的样式停在「提示符里可以塞 ANSI」，画不出框。课上换 Bubble Tea，做成 Claude Code / OpenCode 那种边框输入。

本仓库步骤 2 用 **crossterm 一次性 MVU**（`ChatInputState` + `apply` + `render_box`），不上 ratatui，也不写第 12 课整屏 TUI。`ReadChatInput` 每轮起一次、提交就退。不用 alt-screen，框留在屏幕上。

```text
╭─────────────────────────────────────────╮
│ ❯ your message                          │
╰─────────────────────────────────────────╯
 enter: send · ↑↓: history · ctrl-d: exit
```

`buffer_text`：按 ↑ 之前正在打的字。↓ 越过最新一条时还回去。

## 为什么走两步

- **scanner → readline**：手感立刻好，改动小。
- **readline → MVU**：下一套范式。第 12 课整屏 TUI 还用它。本课只对输入做一次热身。

课上说时间只够一步就做 readline。本仓库两步都落：第 04 课步骤 1，本课步骤 2。

## 两个读者抢 stdin

fancy 输入库和另一处 `stdin.lines()`（比如 `confirm`）不能并存，字节会被抢走。本课：REPL 和 confirm 走同一条 `SessionInput` 读键路。第 12 课整屏程序里 confirm 变成状态转换，这个问题自己消失。

## 本课陷阱

1. **历史落盘。** 聊天里贴过 API key，就进了 `~/.rustbyo_harness_history`。学习项目接受。生产不要持久化，或先打码。
2. **非 TTY。** 课上 `tea.NewProgram` 在管道里会崩。本仓库非 TTY / `BYO_PLAIN_INPUT=1` 仍走 `stdin.lines()`。
3. **多行。** textarea 里方向键是移光标，不是翻历史。本课单行，躲开。

本课不写 textarea，不写第 12 课 TUI。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 08
cargo test --example lesson_08
```

TTY 里应看到边框。↑↓ 翻历史；↓ 回到没按 ↑ 之前的草稿。管道测试不进边框。
