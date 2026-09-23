# 07 · 压缩策略

对照：[Compaction strategies](https://www.byoharness.dev/chapters/07-compaction-strategies.html) · 本课代码：[`examples/07-compaction-strategies.rs`](../../examples/07-compaction-strategies.rs)

第 06 课写明了：每次调用都寄整段对话。对话变长，费用线性涨，窗口也会顶满。

本课第一次必须**扔掉信息**。前面每课都只加。压缩和 Provider 一样要能换：一个接口、多份实现，REPL 里换一行。

## 接口

```text
trait CompactionStrategy {
    fn compact(&self, messages: &[Message], llm: &dyn Provider) -> Result<Vec<Message>, String>;
}
```

课上 `Compact(ctx, messages)`。本仓库把 Provider 从 Summarize 字段挪到参数里，避免会话再持一份 `llm`。策略在 `agent_loop` **每轮开头**调用。多数时候原样返回。

`llm` 只给 Summarize 用。`NoCompaction` / `SlidingWindow` 忽略它。

## 三种策略 + 装饰器

| 策略 | 做什么 | 何时用 |
|---|---|---|
| `NoCompaction` | 原样返回 | 默认 |
| `SlidingWindow { keep_last: N }` | 只留最后 N 条 | 便宜，丢旧上下文 |
| `Summarize { threshold, keep_recent }` | 让模型摘要旧半段，换成一条合成 user | 多一次 API |
| `with_logging(inner, path)` | 包任意策略，长度变了写 before/after | 对比策略 |

```text
// src/repl.rs — 换这一行
let compact = NoCompaction;
// let compact = SlidingWindow { keep_last: 10 };
// let compact = Summarize { threshold: 20, keep_recent: 6, instructions: String::new() };
// let compact = with_logging(SlidingWindow { keep_last: 10 }, ".local/compactions.log");
```

日志只写 `.local/`（gitignore），不要写仓库根。

## 安全切分

天真截断可能把 `tool_use` 丢在扔掉的一侧、`tool_result` 留在留下的一侧。下一轮 API 400，对话救不回来。

`safe_split_point` 从欲切点往回走，找到「带文本的 user、且不是 tool_result」。那是新一轮的起点，切开一定安全。找不到就回 0，什么都不做。

## 摘要

越过 `threshold`：

1. 在 `len - keep_recent` 附近找安全切分点。
2. 旧半段用同一个 Provider 摘要：一次 `send`，不带 tools。
3. 换成 `[earlier conversation summary] ...`，近期半段原样留下。

`threshold: 0` 会强制开火（`/compact summarize`）。摘要吃的是 Provider 上的 system（本仓库自称 DeepSeek）。要换口吻就填 `instructions`。

## 斜杠命令

| 命令 | 作用 |
|---|---|
| `/compact` | 跑当前配置的策略 |
| `/compact sliding` | 当场 `SlidingWindow { keep_last: 6 }` |
| `/compact summarize` | 当场 `Summarize { threshold: 0, keep_recent: 4 }` |
| `/compact none` | 当场 `NoCompaction` |
| `/verbose [on\|off]` | 压缩缩短时打印 before/after |

## 本课陷阱

1. Summarize 调 `Provider::send`。压缩必须在循环里、`send` 外面，否则递归。
2. 摘要继承 system。本仓库是 DeepSeek 编码助手，它会偏向路径和符号。
3. 压缩会改前缀字节。若以后开 prompt cache，缓存寿命等于两次压缩之间。

本课不写 TokenBudget（课上练习），不把 `messages` 搬进 `Agent` 结构体（第 11 课）。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 07
cargo test --example lesson_07
```

单测用 Mock：滑动窗不切开 tool 对、摘要替换旧半段、`/compact` / `/verbose`、循环在 `send` 前压缩。不打网。
