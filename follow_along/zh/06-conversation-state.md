# 06 · 对话状态

对照：[Conversation state](https://www.byoharness.dev/chapters/06-conversation-state.html) · 本课代码：[`examples/06-conversation-state.rs`](../../examples/06-conversation-state.rs)

短的一课。把一直成立但没写明的事钉住：**模型没有记忆。** 每次 API 调用都把整段对话从头寄出。

## 不变量

```text
POST /v1/messages → response     ← 课上 Anthropic
POST /chat/completions → response  ← 本仓库 DeepSeek
```

三次调用彼此独立。没有 session id，服务端不记线程。对话能续上，是 harness 每次把整段切片再寄出去。

## 切片在哪、里面有什么

```text
let mut messages: Vec<Message> = Vec::new();
```

这就是「说过什么」的真相来源。每轮往里写；每次 `send` 读整段。

| 步骤 | 往切片里加什么 | 为什么 |
|---|---|---|
| 你提交一行 | `{Role: User, Content: [Text]}` | 你的话 |
| 模型回复 | `{Role: Assistant, Content: resp}` | 含任何 `tool_use` |
| 工具跑完 | `{Role: User, Content: [ToolResult, ...]}` | 本轮全部结果 |
| 循环再 `send` | 不加新条目 | 循环在切片上转 |

一轮用了一个工具之后是四条：user 文本 → assistant（tool_use）→ user（tool_result）→ assistant 终稿。下一句是第五条。

## `tool_use` 和 `tool_result` 的线

每个 `tool_use` 有一个不透明 id（课上 `toolu_01abc...`）。对应的 `tool_result` 必须带同一个 `tool_use_id`。对不上，真实 API 会 400。id 是模型发的，我们原样回传。

第 07 课压缩如果从一对 `tool_use` / `tool_result` 中间切开，这条线会断。本课只检查配对，不写压缩。

## system 不在切片里

通用 `Role` 没有 `System`。课上 Anthropic 把它放在请求的 `System` 字段；本仓库 DeepSeek 没有那一栏，适配器每次插入第一条 `role: system`。第 03 课的 Provider 把这个差异藏住。模型不把 system 当成「第一条用户消息」。

## 两个后果

### `/clear` 是一行

`messages.clear()`。下一轮 `send` 只带 system，没有先前回合。第 05 课见过，现在知道为什么有效。

### 费用随对话线性涨

每轮都要把前面所有回合再 tokenize 一遍。第 10 轮在为前 9 轮付钱。

两条出路，本课都不实现：

1. **Prompt cache**（第 17 课）：告诉 API 前缀稳定，缓存段大约 10%。生产答案。
2. **压缩**（第 07 课）：旧对话收成摘要，换成摘要再寄。

都是客户端对「服务端无记忆」的反应。服务端压缩（Anthropic 的 `compact-2026-01-12`）是一家专有能力；学习用 harness 要能换 Provider，所以压缩必须在客户端。

## 本课陷阱

1. 搞懂无状态之后想上向量库、检索、上下文管理器。别。整段切片对 coding agent 够用。
2. 迭代切片时改它。循环只读和 append；单线程安全。第 11 课子 agent 才接近要锁。
3. 处理了 `tool_use` 却忘了 append assistant。下一轮孤立的 `tool_result` 会 400。

课上到第 11 课，切片从包级全局搬到 `Agent` 字段。本课仍由 REPL 持有 `Vec<Message>`，不提前搬。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 06
cargo test --example lesson_06
```

单测用 Mock：一轮工具后四条、JSON dump 能对上 id、删掉 assistant 会变成孤立 `tool_result`、`/clear` 清空。不打网。

课上练习「第 1 轮 vs 第 20 轮计时」要 live，默认测试不做。
