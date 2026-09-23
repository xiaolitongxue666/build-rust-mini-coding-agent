# Project Memory (Compact)

1) **课是 Claude，跑的是 DeepSeek** — 原章与 Go 对照课用 Anthropic Messages / Claude 词表。本仓库 live 走 https://api.deepseek.com/chat/completions，默认 deepseek-flash。课上的 tool_use 只留注释和对课，不要做成默认 live 客户端。
2) **实现节奏** — 用户读完第 N 课才实现第 N 课；对照 follow_along/zh/ 与 byoharness 原章。不要提前引入后续课抽象。注释写为什么，标课号。HEAD 里的 Usage / 计价 / debug / shine / Bubble Tea 属于更后的课。
3) **密钥只在 live** — build.sh / 默认 test.sh / check.sh / 默认 test-flow.sh 不读 key。只有 run.sh、test-flow.sh --live、RUN_LIVE=1 才加载。顺序：环境变量 → <home>/.dsh/.credentials.yaml → <home>/.dsh/.env → <home>/.pi/agent/auth.json。字面量 $DEEPSEEK_API_KEY 不是真密钥。密钥不进仓库。
4) **多 OS 兼容、环境独立** — 脚本能跑 Windows / macOS / Linux / WSL，状态不共享。WSL 不读 /mnt/c 或 Windows 用户目录；Windows 不写 WSL /home。每个 distro 各自 rustup / ~/.dsh / ~/.pi。
5) **Windows MSYS HOME** — MSYS2 bash 常把 HOME 设成 /home/<user> 并清掉 USERPROFILE。这是同一台 Windows，不是 WSL。detect_os=windows 时再认 /c/Users/$(id -un)。set -u 下 local 必须赋初值（raw=""）。WSL 即使继承了 USERPROFILE 也不走这条。
6) **系统提示必须写身份** — 模型看不见请求里的 model=。system 挂在 Provider 上并写明 DeepSeek 和当前模型名；DeepSeek 适配器再插成第一条 role: system。
7) **官方 completions 路径** — 用 {base}/chat/completions，不要自行再拼 /v1。reqwest Policy::none()：跟重定向会丢掉 Authorization，官方回 401。这条只活在 DeepSeek 适配器里。
8) **双入口** — 单课 examples/NN-*.rs（[[example]] lesson_NN）；总体 src/main.rs。共享实现只在 src/ 库。run.sh 无参 cargo run，run.sh 01 跑 example。test.sh 总体；test-lessons.sh 单课。新课先 example 再并进库。
9) **代理只是网络** — 本机 127.0.0.1:7890，WSL 走宿主机 nameserver 的 7890。不是把两套家目录或密钥拼在一起。
10) **记忆 SSOT** — 本文件。/summary-memory 走 summary-project-memory gather/validate/apply；不写 ~/.claude-mem。备份在 .project-memory-backups/（gitignore）。不自动改 AGENTS.md，除非用户点名。
11) **第 03 课 Provider** — 循环只认 Send / Model / SetModel 与通用 Message / Block / ToolDef / Response。变量名用 llm。DeepSeekProvider 是唯一知道 Chat Completions 的文件。MockProvider 测循环，不打网。Clone 后的 next 必须共享（Arc<AtomicUsize>），否则 tool 回合会死循环。
12) **第 04 课 UI** — 字标 ANSI Shadow RUSTBYO（66 列，门槛 69）。spinner 只包 llm.send；cargo test 有 RUST_TEST_THREADS 时关掉。rustyline：Esc 取消本回合、Ctrl+C 一次清空两次退出、↑↓ 仅内存历史。不写 HistoryFile，不上 Bubble Tea，不写 /help。
13) **test-flow** — 默认 = check.sh + 键位清单。--live 先 probe-llm，再 BYO_PLAIN_INPUT=1 管道总体 agent 测门和 .local/live-probe 读写。Windows 上 rustyline 对管道也会 Ok，必须 non-TTY 或 BYO_PLAIN_INPUT=1 才走 stdin.lines()。拒绝正文只回给模型，live 断言看文件不是 grep user denied。
