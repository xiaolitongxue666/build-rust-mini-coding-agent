# 11 · 子 Agent

对照：[Subagents](https://www.byoharness.dev/chapters/11-subagents.html) · 本课代码：[`examples/11-subagents.rs`](../../examples/11-subagents.rs)

第 10 课缝都齐了。还缺一件：委托。每次工具调用都落回主对话。为了一个问题读 20 个文件，主切片就多 20 条 tool_result，焦点回不来。

子 agent 是主 agent 拉起的**另一次**循环：自己的窗口、自己的工具子集，最后一条答案当作一条 tool_result 回去。

为什么要做：

- **调查不污染主上下文。** 20 次读压成一行。
- **分工。** 以后可以再挂 code-review；本课只做 `research`。
- **成本。** 结构上能换便宜模型。本课不换。

## 两步重构 + 一个能力

### 1. `Agent` 结构体

课上 `agentLoop` 读包级全局。要同时有根和子，状态必须是值。

字段里把根和子分开的是：`Name`、`LogPrefix`、`Quiet`、`Confirm`（本仓库是 `use_gate`）。

| 根 | 子 agent |
|---|---|
| LogPrefix `""` | `"  ↳ "` |
| Quiet false | true |
| Confirm 问你 | nil（批准父级 `delegate_*` 就算批了里面的） |
| Name `""` | `"research"` |

`Send` 先 append 一条 user，再跑原来的循环。

### 2. `Subagent` 接口

和 Provider / Tool / CompactionStrategy 同一形状。`Run(task) -> 最终文本`。

`Begin` / `Active` 记飞行中的名字。REPL 堵住时 `/subagents` 抓不到进行中的；第 12 课 TUI 才实时画。本课把计数器摆上。

`Research`：`read_file` 子集，`MaxTurns = 10`，每次 `Run` 是新 `Agent`。

### 3. `DelegateTool`

模型看见 `delegate_research`，参数只有 `task`。跑起来打三行：委托头、缩进的 `[tool]`、带耗时的尾。

## `DelegateTool` 放哪

不进 `src/tools/`。课上的环：

```
tool → subagent → agent → tool
```

Rust 同一 crate 模块也能绕出这种方向。胶水放 [`src/delegate.rs`](../../src/delegate.rs)。`Tool` 接口在 `tools`，谁都能 impl。

## 为什么子 agent 不自登记

工具不需要配置，可以 `default_registry()`。子 agent 要 Provider 和子集，那些是运行时的。接线两行一个：登记 Subagent，再挂 DelegateTool。

## 让模型真的去委托

软提示「需要很多读的时候再用」会被理解成「我自己读就行」。课上的修法：system 写死 **READ-ONLY INVESTIGATION 应走 `delegate_research`**，工具描述用祈使句，不要说明书口吻。

本仓库 system 仍必须自称 DeepSeek 并写上当前模型。研究子 agent 的 system 同样带身份，再加调查规则。

## 本课陷阱

1. **同步。** 父级 tool call 堵住直到子 agent 结束。不能扇出两个并行。并行要父级一轮发两个 tool_use，本课不做。
2. **飞行中看不见。** `/subagents` 在 REPL 空闲时只能列出已登记的。
3. **Subset 是策展不是沙箱。** `subset(&["read_file"])` 只是另一份 Registry。真限制要写在 `Execute` 里。

## 本课不动

不上第 12 课整屏 TUI。不写 CodeReview（课上练习）。不拆 `agent → ui` 的 spinner：UI 本课不读 `Active()`，没有环。

## 跑起来

```bash
bash scripts/run.sh 11
/tools          # 应有 delegate_research
/subagents      # 应有 research
```

问「agent loop 定义在哪」且提示对了，应看到委托头、缩进的 `read_file`、一条综合答案。
