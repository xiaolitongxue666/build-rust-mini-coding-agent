# Agent 约定

本仓库是 **harness 学习项目**：按 [byoharness.dev](https://www.byoharness.dev/index.html) 的课程序列，用 Rust 重建同一套分层。目标是看懂每根线，不是一次交付生产系统。

对照：[betta-tech/byo-coding-agent](https://github.com/betta-tech/byo-coding-agent)。课程正文在 `follow_along/zh/`。

**课是 Claude，跑的是 DeepSeek。** 原章用 Anthropic Messages 词表讲循环；本仓库 live 走 `https://api.deepseek.com/chat/completions`（默认 `deepseek-flash`）。系统提示必须自称 DeepSeek 并写上当前模型名。课上的 Claude / `tool_use` 只留在注释和对课里，不要做成默认 live 客户端。

## 实现节奏

用户先自己读课，再让你实现。不要替用户预习后续课，也不要在一回合里把后面几课写完。

1. **未点名的课不要写。** 用户说「读完第 N 课」或「实现第 N 课」之后，只落地第 N 课。
2. **先对课，再改代码。** 打开对应的 `follow_along/zh/NN-*.md` 和 byoharness 原章，按那一课的形状实现；HEAD 里更后的抽象先别提前引入。
3. **注释写为什么。** 中文。写设计取舍、失败模式、与 Go 原课 / DeepSeek 线协议的差异。关键处标课号，例如 `// 第 07 课：截断必须落在干净边界`。不要写复述下一行代码的注释，不要删已有课号注释。
4. **一次少动文件。** 本课能跑、能 `bash scripts/check.sh` 即可。下一课留给下一课。

## 密钥

写代码、`scripts/build.sh`、默认 `scripts/test.sh`、`scripts/check.sh` **不读 API key、不打真实模型**。

只有 `scripts/run.sh` 和 `RUN_LIVE=1 scripts/test.sh` 才加载 DeepSeek 密钥。只读 **当前环境** `$HOME`：环境变量 → `~/.dsh/.credentials.yaml` → `~/.dsh/.env` → `~/.pi/agent/auth.json`。密钥不进仓库。

## 多 OS 兼容，环境独立

脚本在 Windows Git Bash / MSYS、WSL、macOS、Linux 都能跑。**兼容不是共用状态。**

- 每个 OS、每个 WSL distro 各自一套 `$HOME`、toolchain、`~/.dsh` / `~/.pi`。
- 禁止从 WSL 读/写 `/mnt/c`、`C:\`、Windows 用户目录里的密钥或配置；禁止从 Windows 写 WSL `/home`。
- `:7890` 只是网络出口（WSL 走宿主机端口），不是把两套家目录拼在一起。

## 分层

三个扩展点保持正交：`Provider`、`Tool` + `Registry`、`CompactionStrategy`。新能力优先挂进已有缝，不要另起一套平行抽象。

## 例子放哪

官网只给了 **一份** 冻结快照：第 01 课的 `examples/minimal`（看清循环，没有任何后续抽象）。02–19 在同一份 harness 上长，HEAD 才是母本。

本仓库对齐：

- `examples/NN-*.rs`：只在该课需要「无后续抽象的可运行快照」时才加。现在只有 `examples/01-the-agent-loop.rs`。
- `src/`：第 02 课起的生长母本。不要每课再复制一份完整 agent。
- 按课测试：`bash scripts/test-lessons.sh` / `bash scripts/test-lessons.sh 01`。新课实现后改 `scripts/lib/lessons.sh` 的 status，不要靠新 example 凑课号。

## 验证

改完跑 `bash scripts/check.sh`。按课跑 `bash scripts/test-lessons.sh`。默认单测用 Mock / 纯逻辑，不依赖网络。
