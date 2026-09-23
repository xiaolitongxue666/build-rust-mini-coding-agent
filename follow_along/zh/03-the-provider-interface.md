# 03 · Provider 接口

对照：[The provider interface](https://www.byoharness.dev/chapters/03-the-provider-interface.html) · 本课代码：[`examples/03-the-provider-interface.rs`](../../examples/03-the-provider-interface.rs)

到第 02 课为止，循环直接打 DeepSeek Chat Completions。线协议类型（`ChatMessage`、`tool_calls`、`finish_reason`）洒在循环里，harness 绑死一家后端。

本课把 LLM 后端抽成可换的缝。

## 要的形状

```text
var llm Provider = DeepSeekProvider::new(...)
```

加 OpenAI、Bedrock、本地 Ollama、或 Mock，应该是一个新文件加 `main` 里一行。再多就是抽象画错了。

课上的参考适配器是 Anthropic。本仓库 live 走 DeepSeek。本地服务器几乎都暴露 OpenAI 兼容口：有了 DeepSeek / OpenAI 这一份，指向 `http://localhost:11434/v1/` 是构造时的一个选项，不是新包。

## 通用类型是交集

`Message` / `Block` / `ToolDef` / `Response` 是我们自己的类型。接口上不能出现 `ChatMessage` 或 Anthropic SDK 类型，否则调用方仍锁在一家。

块类型按 Anthropic 原生形状最干净。DeepSeek 这边：`tool_use` → `tool_calls`，`tool_result` → 单独的 `role: tool`。翻译只活在适配器里。

system 不进通用 `Role`。Anthropic 放在请求的 `System` 字段；DeepSeek 没有那一栏，适配器插入第一条 `role: system`。循环看不见这个差异。

`Model` / `SetModel` 是给第 05 课 `/model` 的小让步。本课不写斜杠命令。

## 翻译缝

`provider/deepseek.rs` 是 harness 里唯一知道 Chat Completions 的文件。缝正好两处：

1. `to_messages` / `to_tools`：通用 → 线协议
2. `from_choice`：线协议 → 通用 `Response`

SDK 类型漏到循环或工具里，抽象就没做成。

## 本课赚到什么

1. **能换供应商。** 再写一个适配器，`main` 换一行。循环不动。
2. **能不带密钥测循环。** `MockProvider` 返回罐头回复，测循环、门、分发。
3. **同一会话里原则上能跑两个模型。** 第 11 课子 agent 会用到；接口不在乎。

代价是每次 `send` 走一遍翻译。相对网络延迟可忽略。供应商专有旋钮（Anthropic adaptive thinking）留在适配器构造时，不进接口。

## 本课陷阱

1. map 迭代顺序随机，同一套 `InputSchema` 可能编出不同字节，第 06 课 prompt cache 会失效。第 09 课 Registry 再按名字排序。
2. 包名是 `provider` 时，变量不要也叫 `provider`。课上改名为 `llm`。
3. 提前引入 Registry / TUI / `/model`。
4. 在 example 里再抄一份循环——共享实现只在 `src/` 库。

## 跑起来

```bash
bash scripts/run.sh                  # 总体（含本课的 Provider）
bash scripts/run.sh 03               # 本课 demo
cargo run --example lesson_03
cargo test --example lesson_03       # Mock，不打 DeepSeek
```
