# 13 · 接下来

对照：[What's next](https://www.byoharness.dev/chapters/13-whats-next.html)

核心书到第 12 课结束。本课**不加代码**。课表是 `docs`，没有 `examples/13-*.rs`。

你已经有一个能跑的 coding agent：打 DeepSeek、跑工具、破坏性操作先问、调查可以委托、长对话能压缩、TTY 整屏不那么难看。

这一章写的是**故意没做的层**，以及还能挂进现有缝的练习。14–19 是 extras，点名再写。

## 课上说缺什么 · 本仓库对照

| 课上缺的 | 本仓库 |
|---|---|
| 流式输出 | 还没有。`Provider::send` 仍是整段回来再印。 |
| 测试 | 课上说 Go 对照几乎没自动化。本仓库从第 01 课就有 Mock + `check.sh`。 |
| Prompt cache | 第 17 课。压缩会打掉前缀。 |
| 多行输入 | 还是单行。Shift-Enter 在不少终端分不清。 |
| 比「每次都问」更有趣的权限 | 第 02 课草图。`PermissionPolicy` 没写。 |
| MCP | 第 14 课。 |
| 持久化对话 | 没有 `/save` `/load`。历史文件只是输入历史。 |
| Token 计数 | 第 16 课。 |
| TUI 里的 ↑↓ 历史 | 课上练习 8。本仓库第 08 / 12 课已经接上 `$HOME/.rustbyo_harness_history`。 |

## 练习（本课不落地）

课上从易到难：`web_fetch`、斜杠别名、`MockProvider`、`TokenBudget`、`PermissionPolicy`、第二个子 agent `CodeReview`、流式、把第 08 课历史接到 TUI。

本仓库 Mock 和历史已经有了。其余等你点名，或等对应 extras 课。不要在这一课里提前写。

## 三层同一套纪律

| 层 | 动什么 | 时间 |
|---|---|---|
| 构建 | 循环、Provider、Registry、压缩 | ~1% |
| 扩展 | 往已有缝插：MCP、token、新子 agent | ~10% |
| 配置 | AGENTS.md、mcp.json、斜杠模板、提示 | ~89% |

一份自相矛盾的 50KB `AGENTS.md`，手感和一个缝没对上的 harness 一样坏。这本书强调构建，是因为缝看见了，配置才读得懂。

## 收束

骨架都一样：循环、工具面、权限门、上下文、UI。下次用别人的 coding tool，能把怪癖读成 harness 决定。

模型不是产品。Harness 才是。

live 仍是 DeepSeek，不是 Claude。
