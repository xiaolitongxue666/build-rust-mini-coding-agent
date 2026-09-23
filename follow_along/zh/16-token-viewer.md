# 16 · Token 与人民币

对照：[The token viewer](https://www.byoharness.dev/chapters/16-token-viewer.html) · 本课代码：[`examples/16-token-viewer.rs`](../../examples/16-token-viewer.rs)

每次响应带回 usage。以前丢掉了。现在累进同一份账本：`/tokens` 打明细，空闲状态栏打一行。在跑的时候这一行仍是 `thinking...`。

## 接口保持窄

课上 `TotalUsage` 不进 `Provider`。Rust 不能对 `dyn Provider` 做那种类型断言，所以接口上是默认方法 `token_report`：Mock 返回空，DeepSeek 才覆盖。TUI 不拿 provider，REPL 把账本收成闭包，每帧读一次。子 agent 的 clone 共享同一个 `Arc`，研究调用的 token 算进同一笔。

## 按发出那一刻计价

会话可以跨过 12:00 或 18:00。不能等着看 `/tokens` 的时候再按「现在」重算整段。每次 `send` 成功解析出 usage，就按当时的北京时间入高峰桶或空闲桶，人民币累加。

钟点是固定 UTC+8，不读本机时区。Windows、macOS、Linux、WSL 对同一个瞬间落在同一档。

高峰（北京时间，左闭右开）：周一到周五 09:00–12:00、14:00–18:00。周末全天空闲。2026 法定节假日全天空闲，调休上班日按工作日。放假表只认[国办发明电〔2025〕7号](https://www.gov.cn/zhengce/zhengceku/202511/content_7047091.htm)。其它年份不套这张表。

## 人民币

官方价目是美元：<https://api-docs.deepseek.com/quick_start/pricing>。Flash 的人民币档是这张表 × 20/3（空闲：命中 ¥0.02、未命中 ¥1、输出 ¥4；高峰翻倍）。20/3 是核算汇率，不是实时外汇。Pro 用同一乘数。

`deepseek-flash`、旧名 `deepseek-v4-flash`、以及本仓库 `/model` 里的 `deepseek-chat`、`deepseek-reasoner` 按 Flash。`deepseek-v4-pro` 按 Pro。其它名字显示「未知模型」，不显示 ¥0。金额前缀 `~`。

DeepSeek 的 `prompt_tokens = hit + miss`。miss 记入 input，hit 记入 cache read。没有 cache 写入这一档。两边 cache 字段都是 0 时，整段 prompt 按 miss 计。cache 行为 0 的行不打印。

## 跑起来

```bash
bash scripts/run.sh
bash scripts/run.sh 16
bash scripts/test-lessons.sh 16
```

默认测试不打真实模型。实机里问一句，再 `/tokens`。状态栏在空闲时出现同一笔账。
