# 05 · 斜杠命令

对照：[Slash commands](https://www.byoharness.dev/chapters/05-slash-commands.html) · 本课代码：[`examples/05-slash-commands.rs`](../../examples/05-slash-commands.rs)

到第 04 课，REPL 只会把一行送给模型。没有办法问「现在哪个模型」、清对话、干净退出（只能 Ctrl+D / 两次 Ctrl+C）。

本课：以 `/` 开头的行在进模型之前拦截。

## 分发

`run_command` 回 `bool`（本仓库是 `Option<CommandOutcome>`）：处理过就不要再送给模型。未知命令也算处理过，否则 `/asdf` 会进对话。

`SplitN` 只切第一段空格，所以 `/model deepseek-chat` 的参数完整保留。

课上把 `provider` / `messages` 提成包级全局，方便单 goroutine 教学。本仓库用 `CommandCtx` 注入，单测不用全局。

## 命令

| 命令 | 作用 |
|---|---|
| `/help` | 按名字列出 |
| `/model` / `/model name` | 看或改。不校验 id，错了下一轮 API 报错。本仓库建议 `deepseek-flash` / `deepseek-chat` / `deepseek-reasoner`，不是课上的 Claude 名 |
| `/clear` | `messages.clear()`。模型无状态，清本地切片就是清对话框。**不是** Ctrl+C 一次那个输入框 |
| `/tools` | 列出当前工具面 |
| `/exit` | 与 Ctrl+C 两次 / Ctrl+D 同一条 `Quit`，不打 Error |

`/model` 用的是第 03 课就留在 `Provider` 上的 `Model` / `SetModel`。

## 本课陷阱

1. `messages = messages[:0]` 在 Rust 是 `clear()`。不要当成「只清输入框」。
2. 用 `Split` 而不是只切一次，带空格的参数会碎。
3. 命令不要另起一个 crate：它们碰到循环、Provider、工具。放在库的集成层。
4. 提前写 `/compact` / `/save`（后面的课）。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 05
cargo test --example lesson_05
```

先聊两轮，`/clear`，再问「我们刚才说了什么」。模型不应记得。`/exit` 应和两次 Ctrl+C 一样静默离开。
