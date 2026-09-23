# 00 · 这是什么

原课：[byoharness.dev](https://www.byoharness.dev/index.html) · [Go 仓库](https://github.com/betta-tech/byo-coding-agent)

模型写代码。Harness 决定它看见什么、记住什么、能碰什么。本仓库用 Rust 按同一课程序列重建那套脚手架，方便对照阅读。

原课用 Claude / Anthropic Messages 讲 harness。本仓库构建的 agent 实际访问 DeepSeek（Chat Completions，默认 `deepseek-flash`）。写代码在 Cursor 里完成，不消耗 DeepSeek。只有 `bash scripts/run.sh`（以及可选的 `RUN_LIVE=1`）才会加载本机密钥去打真实模型。

单课 demo 在 `examples/NN-*.rs`。总体在 [`src/main.rs`](../../src/main.rs)（`bash scripts/run.sh`）。第 01 课：[`examples/01-the-agent-loop.rs`](../../examples/01-the-agent-loop.rs)（`run.sh 01`）。第 02 课：[`examples/02-the-permission-gate.rs`](../../examples/02-the-permission-gate.rs)（`run.sh 02`）。总体测试：`bash scripts/test.sh`。按课：`bash scripts/test-lessons.sh`。
