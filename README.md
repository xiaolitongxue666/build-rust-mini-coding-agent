# build-rust-mini-coding-agent

用 Rust 按 [Build Your Own Coding Agent](https://www.byoharness.dev/index.html) 重建的学习用 harness。理念和分层对齐 [betta-tech/byo-coding-agent](https://github.com/betta-tech/byo-coding-agent)，课程在 `follow_along/zh/`。

**课是 Claude，跑的是 DeepSeek。** 原章用 Anthropic Messages 讲循环；本仓库构建的 agent 实际访问 `https://api.deepseek.com/chat/completions`（默认 `deepseek-flash`）。系统提示必须自称 DeepSeek。写代码用 Cursor，只有跑自建 agent 时才打真实模型。

Agent 约定：[AGENTS.md](AGENTS.md)。读完一课再实现一课。

## 当前进度

第 12 课已落地。总体（01–12）：[`src/main.rs`](src/main.rs)。TTY 是整屏程序（视口 + 输入 + 审批 + 转圈）；管道 / `BYO_PLAIN_INPUT=1` 仍走 `stdin.lines()`。单课 demo：[`examples/01-the-agent-loop.rs`](examples/01-the-agent-loop.rs) … [`examples/12-full-tui.rs`](examples/12-full-tui.rs)。

## 脚本

兼容 Windows Git Bash / MSYS、PowerShell、macOS、Linux、WSL。各 OS / 各 WSL distro 的 `$HOME`、密钥、toolchain 完全隔离：WSL 不读 `/mnt/c`，Windows 不写 WSL `/home`。

Windows 上 MSYS 的 `$HOME` 经常是 `/home/<user>`，和 `C:\Users\<user>` 是同一台 Windows，不是 WSL。代理只是网络：本机 `127.0.0.1:7890`，WSL 走宿主机 `7890`。

```bash
bash scripts/bootstrap.sh         # 安装 rustup / rustfmt / clippy
bash scripts/build.sh
bash scripts/test.sh              # 总体（库 + bin），不读密钥
bash scripts/test-lessons.sh      # 单课 example；可跟 01 / list
bash scripts/check.sh             # 离线：fmt + clippy + test + 单课
bash scripts/test-flow.sh         # 离线 check + 键位手测清单
bash scripts/test-flow.sh --live  # 再加上管道实机：权限门 + .local/live-probe 读写
bash scripts/probe-llm.sh         # 打一次官方 completions，确认密钥（不打印 key）
bash scripts/run.sh               # 总体 src/main.rs
bash scripts/run.sh 01            # 第 01 课 demo
bash scripts/run.sh 02            # 第 02 课 demo
bash scripts/run.sh 03            # 第 03 课 demo
bash scripts/run.sh 04            # 第 04 课 demo
bash scripts/run.sh 05            # 第 05 课 demo
bash scripts/run.sh 06            # 第 06 课 demo
bash scripts/run.sh 07            # 第 07 课 demo
bash scripts/run.sh 08            # 第 08 课 demo
bash scripts/run.sh 09            # 第 09 课 demo
bash scripts/run.sh 10            # 第 10 课 demo
bash scripts/run.sh 11            # 第 11 课 demo
bash scripts/run.sh 12            # 第 12 课 demo
```

Windows PowerShell：`.\scripts\bootstrap.ps1`

`run.sh` 只认 **当前环境**：`DEEPSEEK_API_KEY` → `<home>/.dsh/.credentials.yaml` → `<home>/.dsh/.env` → `<home>/.pi/agent/auth.json`。Windows 还会看本机 `C:\Users\<user>`；WSL 只看自己的 `$HOME`。在这一侧用 [agent-config](../agent-config) apply，不要写进本仓库，也不要跨 OS 借密钥。字面量 `$DEEPSEEK_API_KEY` 不是真密钥。
