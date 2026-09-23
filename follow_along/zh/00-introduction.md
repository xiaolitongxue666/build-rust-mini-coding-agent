# 00 · 这是什么

原课：[byoharness.dev](https://www.byoharness.dev/index.html) · [Go 仓库](https://github.com/betta-tech/byo-coding-agent)

模型写代码。Harness 决定它看见什么、记住什么、能碰什么。本仓库用 Rust 按同一课程序列重建那套脚手架，方便对照阅读。

原课用 Claude / Anthropic Messages 讲 harness。本仓库构建的 agent 实际访问 DeepSeek（Chat Completions，默认 `deepseek-flash`）。写代码在 Cursor 里完成，不消耗 DeepSeek。只有 `bash scripts/run.sh`（以及可选的 `RUN_LIVE=1`）才会加载本机密钥去打真实模型。

第 01 课快照：[`examples/01-the-agent-loop.rs`](../../examples/01-the-agent-loop.rs)。官网也只在这一课放独立例子；02 起写进 `src/`。按课测试：`bash scripts/test-lessons.sh`。
