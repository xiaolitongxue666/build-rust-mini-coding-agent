# 04 · UI polish

对照：[UI polish](https://www.byoharness.dev/chapters/04-ui-polish.html) · 本课代码：[`examples/04-ui-polish.rs`](../../examples/04-ui-polish.rs)

到第 03 课，REPL 能跑，但提示只是 `>`，等模型时一片空白。本课加纹理：字标、窄终端回退、等待 spinner。这些不承重，是 CLI 里到处会碰到的三件小技术。

## 字标

课上用 ANSI Shadow 写 BETTATECH。本仓库字标是 **RUSTBYO**，同样的字体、粗体青 `\033[1;36m`、暗灰副标题 `rust mini coding agent · DeepSeek`。

字标 66 列。低于 69 列（留 3 列呼吸空间）或 stdout 不是 TTY 时，改打单行 `RUSTBYO`。管道到文件时 `GetSize` 失败当 0 列，走小字标。

## spinner

只包住 `llm.send`。帧是 `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`，80ms 一格。`\r` 回到列 0，`\033[K` 清到行尾。`Stop` 必须等到线程确认清行，再打模型文本。非 TTY 是空壳。

第 12 课会换成 TUI 里的 spinner。本课这版够 REPL 用。

## 本仓额外键位

课上没有这些。本课薄输入（rustyline，不是 Bubble Tea）：

- **Esc**（任务进行中）：取消这一回合，回到 `>`，不退出进程。
- **Ctrl+C 一次**：清空输入框，不报错。
- **Ctrl+C 两次** 或 **Ctrl+D**：静默退出。
- **↑ / ↓**：本进程提交过的行。不写磁盘历史。

confirm 必须和 REPL 共用这把 editor。再 lock 一次 stdin 会抢字节。

## 本课陷阱

1. 把时间戳或动画写进 system prompt，第 06 课 prompt cache 会失效。字标只打一次，不进 messages。
2. 有的终端里 `█` / braille 不是 1 格宽，字标可能折行。接受。
3. 提前引入 `/help`、shine 动画、Bubble Tea。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 04               # 本课 demo
cargo test --example lesson_04       # 不打 DeepSeek
```

把终端缩到 60 列再启动，应看到单行字标。`bash scripts/run.sh 04 > /tmp/out.txt` 打开文件也应是小字标。
