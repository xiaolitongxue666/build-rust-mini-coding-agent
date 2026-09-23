# build-rust-mini-coding-agent

用 Rust 按 [Build Your Own Coding Agent](https://www.byoharness.dev/index.html) 重建的学习用 harness。理念和分层对齐 [betta-tech/byo-coding-agent](https://github.com/betta-tech/byo-coding-agent)，课程在 `follow_along/zh/`。

**课是 Claude，跑的是 DeepSeek。** 原章用 Anthropic Messages 讲循环；本仓库构建的 agent 实际访问 `https://api.deepseek.com/chat/completions`（默认 `deepseek-flash`）。系统提示必须自称 DeepSeek。写代码用 Cursor，只有跑自建 agent 时才打真实模型。

Agent 约定：[AGENTS.md](AGENTS.md)。读完一课再实现一课。

## 当前进度

第 01 课已落地：[`examples/01-the-agent-loop.rs`](examples/01-the-agent-loop.rs) — 官网同款的冻结快照（REPL + DeepSeek 循环 + 三个工具）。生长母本从第 02 课起在 `src/`。

## 脚本

兼容 Windows Git Bash / MSYS、PowerShell、macOS、Linux、WSL。各 OS / 各 WSL distro 的 `$HOME`、密钥、toolchain 完全隔离：WSL 不读 `/mnt/c`，Windows 不写 WSL `/home`。

Windows 上 MSYS 的 `$HOME` 经常是 `/home/<user>`，和 `C:\Users\<user>` 是同一台 Windows，不是 WSL。代理只是网络：本机 `127.0.0.1:7890`，WSL 走宿主机 `7890`。

```bash
bash scripts/bootstrap.sh         # 安装 rustup / rustfmt / clippy
bash scripts/build.sh
bash scripts/test.sh              # 默认不读密钥
bash scripts/test-lessons.sh      # 已实现的课；可跟 01 / list
bash scripts/check.sh
bash scripts/probe-llm.sh         # 打一次官方 completions，确认密钥（不打印 key）
bash scripts/run.sh               # 加载 DeepSeek 密钥后启动第 01 课快照
```

Windows PowerShell：`.\scripts\bootstrap.ps1`

`run.sh` 只认 **当前环境**：`DEEPSEEK_API_KEY` → `<home>/.dsh/.credentials.yaml` → `<home>/.dsh/.env` → `<home>/.pi/agent/auth.json`。Windows 还会看本机 `C:\Users\<user>`；WSL 只看自己的 `$HOME`。在这一侧用 [agent-config](../agent-config) apply，不要写进本仓库，也不要跨 OS 借密钥。字面量 `$DEEPSEEK_API_KEY` 不是真密钥。
