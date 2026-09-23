# Project Memory (Compact)

1) **课是 Claude，跑的是 DeepSeek** — 原章用 Anthropic Messages / Claude 词表。live 走 https://api.deepseek.com/chat/completions，默认 deepseek-flash。tool_use 只留注释和对课，不要做成默认 live 客户端。
2) **实现节奏** — 用户读完第 N 课才实现第 N 课。对照 follow_along/zh/ 与 byoharness。不要提前写 13+ 或 HEAD 里的 Usage / 计价 / debug / shine / MCP / token 栏 / ratatui。
3) **密钥只在 live** — build / 默认 test / check / 默认 test-flow 不读 key。只有 run.sh、test-flow.sh --live、RUN_LIVE=1 才加载。顺序：环境变量 → <home>/.dsh/.credentials.yaml → <home>/.dsh/.env → <home>/.pi/agent/auth.json。不进仓库。
4) **多 OS、环境独立** — Windows / macOS / Linux / WSL 能跑，状态不共享。WSL 不读 /mnt/c；Windows 不写 WSL /home。MSYS HOME 常是 /home/<user>，detect_os=windows 时再认 /c/Users/$(id -un)。set -u 下 local 先赋 raw=""。
5) **系统提示必须写身份** — 模型看不见 model=。system 挂在 Provider / Agent 上，写明 DeepSeek 和当前模型；适配器插第一条 role: system。只读调查应走 delegate_research（祈使句，不要软提示）。
6) **官方 completions** — {base}/chat/completions，不要再拼 /v1。reqwest Policy::none()，跟重定向会丢 Authorization。只活在 DeepSeek 适配器。chat 是 pub(crate)。
7) **双入口** — examples/NN-*.rs（lesson_NN）+ src/main.rs。实现只在 src/。run.sh 无参总体，run.sh NN 单课。test.sh 总体；test-lessons.sh 单课；check.sh 两路。
8) **代理只是网络** — 本机 127.0.0.1:7890，WSL 走宿主机 7890。不是共用家目录或密钥。
9) **记忆 SSOT** — 本文件。/summary-memory 走 summary-project-memory；不写 ~/.claude-mem。备份 .project-memory-backups/（gitignore，留 2 份）。不自动改 AGENTS.md。
10) **第 03 课 Provider** — 循环只认 Send / Model / SetModel 与通用 Message / Block / ToolDef / Response。变量名 llm。DeepSeekProvider 是唯一知道线协议的文件。Mock 的 next 必须 Arc 共享，否则 tool 回合死循环。
11) **第 05–07 课** — 斜杠用 CommandCtx 不写 static。messages 切片是唯一真相，system 不进切片。压缩在 agent_loop 每轮开头，不要套进 Provider::send。SafeSplitPoint 找不到干净边界就回 0。
12) **第 08–09 课** — rustyline 已替换。TTY 先是一次性 crossterm 框，现为第 12 课整屏。历史写当前 $HOME/.rustbyo_harness_history。工具 Registry + OnceLock；加工具只改 tools/mod.rs 一行。Definitions 按名排序。
13) **第 10 课布局** — Cargo lib+bin+examples，不要套 Go src/internal/。publish=false + 模块默认私有。api 在底。DelegateTool 不进 tools/，以免 tools→subagent→agent→tools。
14) **第 11 课子 agent** — Agent 结构体；Research 只有 read_file 子集（策展不是沙箱）。子 agent 要 Provider，接线时登记，不 OnceLock。Confirm=nil 自动过；Quiet + LogPrefix 区分根/子。
15) **第 12 课整屏** — TTY 一个 crossterm 程序（不上 ratatui/Bubble Tea）。循环在后台线程；println! 进管子；审批走通道。agent 不再 import ui。管道 / BYO_PLAIN_INPUT=1 必须 stdin.lines()。
16) **test-flow** — 默认 check.sh + 键位清单。--live 先 probe-llm，再管道测门和 .local/live-probe。拒绝正文只回给模型，断言看文件。git-smart-commit 在 agent 子 shell 要 --yes。
