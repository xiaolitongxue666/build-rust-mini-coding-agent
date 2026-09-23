# 02 · 权限门

对照：[The permission gate](https://www.byoharness.dev/chapters/02-the-permission-gate.html) · 本课代码：[`examples/02-the-permission-gate.rs`](../../examples/02-the-permission-gate.rs)

和 [第 01 课](01-the-agent-loop.md) 一样是独立小 demo。实现在 crate 库里一份；本文件只接线带门的 REPL。总体 `src/main.rs` 也走这道门。

第 01 课快照 **没有**审批。模型要跑的命令会立刻执行。`ls` 没事，`rm -rf` 不行。

## 审批放哪

| 位置 | 样子 | 取舍 |
|---|---|---|
| 工具内部 | bash 自己问「你确定吗」 | 每个工具都要知道有人在键盘上；组不起来 |
| **harness 层** | `execute_tool` 在分发前问 | 一处、体验一致、工具不用配合。本课选这个 |
| 模型层 | 系统提示里写「先问再做」 | 不可靠。模型是来帮忙的，不是来看门的 |

具体落点：打印 `[tool] …` 之后、真正 `match` 分发之前，调用 `confirm("approve?")`。

原课用 Claude 讲循环。本仓库 live 仍走 DeepSeek；权限门与供应商无关。

## 循环里多的那一步

```
[调用模型] → 有 tool_calls? → 打印 [tool]
    → approve? [y/n]
        n → append "user denied this tool call" (is_error) → 再调模型
        y → 真正跑工具 → append 结果 → 再调模型
```

第 01 课的两个出口没变。只是工具结果现在也可能是「人拒绝了」。

## `confirm`：默认否，共用一行读取器

只认 `y` / `yes`（忽略大小写和两侧空白）。空行、Ctrl-D、任何别的字符都是 **否**。马上要跑 shell 时，保守默认才安全。

Go 课用同一个全局 `bufio.Scanner`。Rust 没有全局 scanner：外层 REPL 的 `stdin.lock().lines()` 必须传进 `confirm`。再 `io::stdin().lines()` 一次会和 REPL 抢字节。

课上不处理「stdin 不是 TTY」。CI 里提示会挂起。生产版应在非 TTY 时自动拒绝；那不是本课。第 12 课才会把 `confirm` 收进 TUI modal，合同不变。

## 拒绝是 tool result，不是崩循环

拒绝时返回 `("user denied this tool call", true)`。`true` 是 `is_error`，和「文件不存在」「bash 非 0」走同一条合同。

不崩溃、不中断对话、不绕过模型。模型读到失败结果后自己换路、改问你、或停下来。**模型在循环里；失败是下一轮的输入，不是异常。**

## 本课闸所有工具

`bash` / `read_file` / `write_file` 每一次都问。对后两个偏谨慎。更细的 `PermissionPolicy`（AlwaysAllow / AllowList / AskOnce）课上留作练习。本课不写那层接口，也不抽 Provider。

## 本课陷阱

1. 给审批另开一套 stdin 读取器。
2. 拒绝时忘了标 `is_error`。
3. 提前引入 Provider / Registry / Policy。
4. 在 example 里再抄一份循环——共享实现只在 `src/` 库。

## 跑起来

```bash
bash scripts/run.sh                  # 总体（含本课的门）
bash scripts/run.sh 02               # 本课 demo
cargo run --example lesson_02
cargo test --example lesson_02       # 本课单测，不打 DeepSeek
```

在 `>` 后让它删点什么。先 `y` 看它实际跑了哪条命令，再同样提示回 `n`，看模型怎么收场。
