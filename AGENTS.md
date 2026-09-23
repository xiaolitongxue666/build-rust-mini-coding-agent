# Agent 约定

本仓库是 **harness 学习项目**：按 [byoharness.dev](https://www.byoharness.dev/index.html) 的课程序列，用 Rust 重建同一套分层。目标是看懂每根线，不是一次交付生产系统。

对照：[betta-tech/byo-coding-agent](https://github.com/betta-tech/byo-coding-agent)。课程正文在 `follow_along/zh/`。

**课是 Claude，跑的是 DeepSeek。** 原章用 Anthropic Messages 词表讲循环；本仓库 live 走 `https://api.deepseek.com/chat/completions`（默认 `deepseek-flash`）。系统提示必须自称 DeepSeek 并写上当前模型名。课上的 Claude / `tool_use` 只留在注释和对课里，不要做成默认 live 客户端。

## 实现节奏

用户先自己读课，再让你实现。不要替用户预习后续课，也不要在一回合里把后面几课写完。

1. **未点名的课不要写。** 用户说「读完第 N 课」或「实现第 N 课」之后，只落地第 N 课。
2. **先对课，再改代码。** 打开对应的 `follow_along/zh/NN-*.md` 和 byoharness 原章，按那一课的形状实现；HEAD 里更后的抽象先别提前引入。
3. **注释对课程页面。** 中文写「为什么」。文件头对应该章标题和原章链接；课上的图、词表、步骤、陷阱要在对应代码旁能对上。关键处标课号，例如 `// 第 02 课：拒绝必须是 is_error`。不要写复述下一行的注释，不要删已有课号注释。
4. **双入口、一份实现。** 单课小 demo 在 `examples/NN-<slug>.rs`；总体在 `src/main.rs`（对齐官网 `main.go`）。共享循环/工具只写在 `src/` 库里，example 与 bin 只接线。不要再复制一整份 agent。
5. **一次少动文件。** 本课能跑、能 `bash scripts/check.sh` 即可。下一课留给下一课。

## 密钥

写代码、`scripts/build.sh`、默认 `scripts/test.sh`、`scripts/check.sh`、默认 `scripts/test-flow.sh` **不读 API key、不打真实模型**。

只有 `scripts/run.sh`、`scripts/test-flow.sh --live`、`RUN_LIVE=1 scripts/test.sh` 才加载 DeepSeek 密钥。只读 **当前环境** `$HOME`：环境变量 → `~/.dsh/.credentials.yaml` → `~/.dsh/.env` → `~/.pi/agent/auth.json`。密钥不进仓库。

## 多 OS 兼容，环境独立

脚本在 Windows Git Bash / MSYS、WSL、macOS、Linux 都能跑。**兼容不是共用状态。**

- 每个 OS、每个 WSL distro 各自一套 `$HOME`、toolchain、`~/.dsh` / `~/.pi`。
- 禁止从 WSL 读/写 `/mnt/c`、`C:\`、Windows 用户目录里的密钥或配置；禁止从 Windows 写 WSL `/home`。
- `:7890` 只是网络出口（WSL 走宿主机端口），不是把两套家目录拼在一起。

## 分层

三个扩展点保持正交：`Provider`、`Tool` + `Registry`、`CompactionStrategy`。新能力优先挂进已有缝，不要另起一套平行抽象。

第 10 课：布局按 [Cargo 包](https://doc.rust-lang.org/cargo/guide/project-layout.html)（`src/lib.rs` + `src/main.rs` + `examples/`），不要套 Go 的 `src/internal/`。`publish = false` + 模块默认私有就是「不是对外 API」。线协议 `chat` 用 `pub(crate)`。`api` 在依赖栈最底。

## 例子与总体

官网只冻 `examples/minimal`，`main.go` 接线全功能。本仓库：

- **单课 demo**：`examples/NN-<slug>.rs` + `[[example]] name = "lesson_NN"`（`test = true`）。对应该章页面写详细注释；只接线本课形状（01 无门，02 有门）。
- **总体**：`src/main.rs` 把已完成课接到一起。实现在 `src/lib.rs` 各模块，不在 example 里再抄一份。
- **跑**：`bash scripts/run.sh` → `cargo run`（总体）；`bash scripts/run.sh 01` → `cargo run --example lesson_01`。
- **测**：`bash scripts/test.sh` → 总体 `cargo test`；`bash scripts/test-lessons.sh [NN]` → 单课 example。`check.sh` 两路都跑。
- **完整流程**：`bash scripts/test-flow.sh` = `check.sh` + 键位手测清单；`bash scripts/test-flow.sh --live` 再管道实机测权限门和 `.local/live-probe` 读写。

新课：先加 example 快照，再把该课能力并进库 + `main.rs`。不要提前引入后续课抽象。

## 验证

改完跑 `bash scripts/check.sh`。要门 + 真实读写时再 `bash scripts/test-flow.sh --live`。按课跑 `bash scripts/test-lessons.sh`。默认单测用 Mock / 纯逻辑，不依赖网络。

管道 / `BYO_PLAIN_INPUT=1` 必须走 `stdin.lines()`：TTY 边框输入（第 08 课）在管道里没有终端。Esc / Ctrl+C / ↑↓ / 边框只能手测（`bash scripts/run.sh`）。live 临时文件只放 `.local/`（gitignore），不要写进仓库根。历史文件写当前 `$HOME/.rustbyo_harness_history`。
