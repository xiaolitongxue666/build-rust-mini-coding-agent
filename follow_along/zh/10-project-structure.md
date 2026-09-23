# 10 · 项目结构

对照：[Project structure](https://www.byoharness.dev/chapters/10-project-structure.html) · 本课代码：[`examples/10-project-structure.rs`](../../examples/10-project-structure.rs)

课上到第 09 课，Go 仓库顶层十几个文件，全是 `package main`。能跑，但新人要把文件自己对上概念。

那一章把东西搬进 `internal/`。Go 里 **扁平往往更地道**；他们为了「克隆下来就能按文件夹认概念」故意拆。本仓库是 **Rust**，不要照搬目录名。

## 官方 Rust 怎么放文件

[Cargo 包布局](https://doc.rust-lang.org/cargo/guide/project-layout.html)：

| 路径 | 角色 |
|---|---|
| `Cargo.toml` | 包根 |
| `src/lib.rs` | 默认库 |
| `src/main.rs` | 默认可执行文件 |
| `examples/` | 例子（本仓库单课 demo） |
| `tests/` | 集成测试（本仓库单测主要在库和 example 里） |

[模块与可见性](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)：从 crate 根声明模块；默认私有；`pub` 才对外。模块按**领域**分组，不按「全是 types.rs / utils.rs」分组。

## 课上 Go → 本仓库

| 课上 | 本仓库 | 为什么不照抄 |
|---|---|---|
| `internal/`（编译器禁止外模块 import） | `publish = false` + 模块默认私有 + `pub(crate)` | Cargo 没有 `internal/` 关键字；再套 `src/internal/` 违反 Cargo 约定 |
| `internal/api` | `src/api.rs` | 依赖栈最底，不 `use crate::` 其它模块 |
| `internal/provider` | `src/provider/` | 换供应商改这里；线协议只在适配器 |
| `internal/tool` | `src/tools/` | 一文件一工具，登记不进 main |
| `internal/compact` | `src/compact/` | 一文件一策略 |
| `internal/ui` | `src/ui/` | banner / spinner / 一次性输入 |
| `main.go` + `commands.go` 留在顶层 | `src/main.rs` 接线；`repl` / `commands` / `gate` 在库里 | 命令碰到所有扩展点，留在集成层 |
| `go mod` 改成 GitHub 路径 | crate 名已是仓库名 | 不必为了对齐 Go 改 package 名 |
| 拆 `internal/provider/anthropic/` 子包 | 不拆 `provider/deepseek/` 子 crate | 和课上一样：拆子包会毁掉「丢一个文件就出现」 |

架构没变。三个扩展点（`Provider`、`Tool`+`Registry`、`CompactionStrategy`）本来就正交，现在也只是目录上能看见。

## 本课对 Rust 动了什么

不把树搬进 `src/internal/`，不拆 workspace（那是多 crate，不是这一课）。

1. `lib.rs` 写明模块树和依赖方向。
2. `chat`（线协议）改成 `pub(crate)`：循环和 example 看不见。对齐课上「只有一处依赖供应商 SDK」。
3. `api` 继续不依赖本 crate 其它模块。

`agent` 现在会用 `ui::spin_until`。这是单向的，没有环。课上的环是 `agent → ui → subagent → agent`，第 11 课才有。本课不提前拆 spinner。

## 本课陷阱（Go 原文 + Rust 对照）

1. **导入环。** Rust 同样怕 `mod` 环。规则：`api` 在底；逻辑认 api；UI 认逻辑，不要反向。
2. **Delegate 放哪。** 第 11 课才有。有时不该放进「看起来对」的包，以免 `tools → subagent → agent → tools`。
3. **测试按模块。** `#[cfg(test)]` 在各模块里；example 测对外缝。不要为了对齐 Go 再开一套 `tests/` 集成树。

## 跑起来

```bash
bash scripts/run.sh                  # 总体
bash scripts/run.sh 10
cargo test --example lesson_10
```

不看 `main.rs` 也应能在一分钟内指出：循环在 `agent` / `repl`，工具在 `src/tools/`，线协议翻译在 `provider/deepseek.rs`。
