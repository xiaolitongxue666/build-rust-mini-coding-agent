# 01 · Agent 循环

对照：[The agent loop](https://www.byoharness.dev/chapters/01-the-agent-loop.html) · 本课代码：[`examples/01-the-agent-loop.rs`](../../examples/01-the-agent-loop.rs)

HEAD 以后的 `src/` 会拆成 Provider、Registry、TUI。这一课只看循环形状，不要去对 `main.rs`。

## 两层循环

```
外层 REPL（你的回车驱动）
    读一行 → 交给内层 → 等下一行

内层 agent loop（模型的选择驱动）
    调模型 → 若有 tool_calls 则执行、append、再调 → 直到模型停
```

游戏循环每秒 60 次；REPL 按你的输入走。内层的「update」是模型 + 工具，不是物理引擎。

## 词表（DeepSeek / OpenAI 兼容）

原课用 Anthropic Messages。本仓库 live 走 DeepSeek 的 Chat Completions。名字不同，角色相同。模型看不见请求里的 `model=`，系统提示必须写明自己是 DeepSeek，否则「你是谁」会自称 Claude。

| 原课（Anthropic） | 本仓库（DeepSeek） | 含义 |
|---|---|---|
| `messages` | `messages` | 到目前为止的全部对话，客户端带着走 |
| `tools` + JSON Schema | `tools` + `function.parameters` | 模型能调用的操作面 |
| `content` 里的 `tool_use` | `message.tool_calls` | 模型要 harness 跑某个工具 |
| `stop_reason: tool_use` | `finish_reason: tool_calls` | 内层继续转 |
| `stop_reason: end_turn` | `finish_reason: stop` | 打出文本，回到 REPL |
| `tool_result` + `tool_use_id` | `role: tool` + `tool_call_id` | 必须对上 id，否则 API 400 |

## 为什么是三个工具，不是一个 bash

`bash` 能读写文件。单独拆出 `read_file` / `write_file` 是为了给 harness 一个可拦截的缝：后面的权限门、diff 审批都挂在具体工具名上。只留 bash 时，审批看到的是不透明命令串。工具面形状是 harness 决策，模型不在乎你给一个还是三个。

## 错误是 tool result，不是 Rust Err

`execute_tool` 返回 `(String, bool)`。路径不存在时，模型读到 `"no such file or directory"`，可以换路径或告诉你。如果这里 `unwrap` 崩循环，模型没有恢复通道。第 02 课会把拒绝也做成同一条合同。

## 本课陷阱

1. 忘记把 assistant 消息（含 `tool_calls`）append 回去。
2. tool 消息的 `tool_call_id` 对不上。
3. 用 `finish_reason == "stop"` 当退出条件，漏掉其它非 `tool_calls` 的结束原因。

## 跑起来

```bash
bash scripts/run.sh
```

在 `>` 后试：`list the files here`、`write a hello.txt with a haiku in it`、`read the file /does/not/exist`。
