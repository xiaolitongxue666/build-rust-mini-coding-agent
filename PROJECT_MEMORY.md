# Project Memory (Compact)

1) **课是 Claude，跑的是 DeepSeek** — 原章与 Go 对照课用 Anthropic Messages / Claude 词表。本仓库 live 走 https://api.deepseek.com/chat/completions，默认 deepseek-flash。课上的 tool_use 只留注释和对课，不要做成默认 live 客户端。

2) **实现节奏** — 用户读完第 N 课才实现第 N 课；对照 follow_along/zh/ 与 byoharness 原章。不要提前引入后续课抽象。注释写为什么，标课号。

3) **密钥只在 live** — scripts/build.sh / 默认 test.sh / check.sh 不读 key。只有 scripts/run.sh 与 RUN_LIVE=1 才加载。顺序：环境变量 → <home>/.dsh/.credentials.yaml → <home>/.dsh/.env → <home>/.pi/agent/auth.json。字面量 $DEEPSEEK_API_KEY 不是真密钥。密钥不进仓库。

4) **多 OS 兼容、环境独立** — 脚本能跑 Windows / macOS / Linux / WSL，状态不共享。WSL 不读 /mnt/c 或 Windows 用户目录；Windows 不写 WSL /home。每个 distro 各自 rustup / ~/.dsh / ~/.pi。

5) **Windows MSYS HOME** — MSYS2 bash 常把 HOME 设成 /home/<user> 并清掉 USERPROFILE。这是同一台 Windows，不是 WSL。detect_os=windows 时再认 /c/Users/$(id -un)。set -u 下 local 必须赋初值（raw=""），否则搜不到用户目录。WSL 即使继承了 USERPROFILE 也不走这条。

6) **系统提示必须写身份** — 模型看不见请求里的 model=。只写 coding assistant 时会自称 Claude。system 里写明 DeepSeek 和当前 LLM_MODEL。

7) **官方 completions 路径** — 用 {base}/chat/completions，不要自行再拼 /v1。reqwest Policy::none()：跟重定向会丢掉 Authorization，官方回 401。

8) **例子布局** — 官网只冻第 01 课一份无抽象快照：examples/01-the-agent-loop.rs（crate example lesson_01）。02-19 长在 src/。按课测试改 scripts/lib/lessons.sh 的 status，不要每课复制一份 agent。

9) **代理只是网络** — 本机 127.0.0.1:7890，WSL 走宿主机 nameserver 的 7890。不是把两套家目录或密钥拼在一起。

10) **记忆 SSOT** — 本文件。/summary-memory 走 summary-project-memory gather/validate/apply；不写 ~/.claude-mem。备份在 .project-memory-backups/（gitignore）。不自动改 AGENTS.md。
