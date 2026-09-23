# 09 · 可插拔工具

对照：[Plug-and-play tools](https://www.byoharness.dev/chapters/09-plug-and-play-tools.html) · 本课代码：[`examples/09-plug-and-play-tools.rs`](../../examples/09-plug-and-play-tools.rs)

第 03 课抽了 LLM，第 07 课抽了压缩。工具还是两截：发给模型的 `[]ToolDef`，和 `execute` 里的 `switch`。加一个工具要改两处。

本课把它们收成一套。

## 又是那个形状，多一个弯

```text
接口 → 可换的实现 → 一处接线
```

工具是**可叠加**的（能同时有很多），不是互斥的（Provider / 压缩各一个）。所以不是 `main` 换一行，而是 Registry。

```text
trait Tool {
    fn definition(&self) -> ToolDef;
    fn execute(&self, input: &str) -> (String, bool);
}
```

课上 `Execute` 带 `context.Context`。本仓库没有那套取消惯用法，签名保持 `(正文, is_error)`。

循环只认 `definitions()` 和 `execute()`，不知道有哪些工具。

## 登记

Go 用同包 `init()`：丢一个文件进去就出现，**不用改 main.go**。

Rust 没有包级 `init()`。同一模块里 `default_registry()` 登记三个工具。加工具：

1. 在 `src/tools/` 丢一个文件，实现 `Tool`。
2. 在 `src/tools/mod.rs` 的 `build_default` 加一行 `register`。

不动 `src/main.rs`。不要拆成 `tools/bash/` 子包，否则又要在某处 `mod` 一遍。

需要 `Provider` / 配置的工具不能自动登记，得在接线处 `register`（第 11 课）。

## 门变薄

`execute_gated` / `execute_direct` 只打 `[tool]`、问 approve，然后 `registry.execute`。

| 关切 | 谁管 |
|---|---|
| 日志 | gate（`[tool] …`） |
| 审批 | gate（confirm） |
| 分发 | Registry |

## 本课陷阱

1. **HashMap 迭代顺序随机。** `definitions()` 必须先按名字排序，否则两次请求字节不同，以后的 prompt cache 会失效。
2. **`Default` 是全局。** 不要往上面挂状态。工具保持小。
3. **同名。** `register` 后写覆盖先写。课上练习让你想查重放哪；本课不写查重。

本课不写 `Subset`（第 11 课），不写 `web_fetch`（课上练习）。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 09
cargo test --example lesson_09
```

`/tools` 应按名字列出 `bash`、`read_file`、`write_file`。
