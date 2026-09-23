//! 总体入口：已完成课接到一起（01–12，以及第 14–18 课）。第 13 课是收束文档，不加代码。
//! 第 10 课：不要在这里堆领域文件；领域在库模块里。
//! 第 11 课：`register_subagents` 在 `repl`，`DelegateTool` 在 `delegate.rs`。
//! 第 12 课：TTY 整屏在 `ui::run_tui`；管子和备用屏也在那里。
//! 第 14 课：MCP 在 `repl::run_repl` 里登记，赶在子 agent 和 stdout 管子之前。
//! 第 15 课：同一处把 cwd 的 `AGENTS.md` 焊进 system。
//! 第 16 课：`/tokens` 和空闲状态栏读同一份人民币账本。
//! 第 17 课：DeepSeek 磁盘缓存默认开着。system 放在第一条，工具按名字排序，压缩不改 system。
//! 第 18 课：本地 `write_file` 在落盘前把统一 diff 交给同一次 y/n。
//! 对齐 [byo-coding-agent](https://github.com/betta-tech/byo-coding-agent) 的 `main.go` 接线角色。
//! 第 03 课：这里换一行就能换供应商；默认 `DeepSeekProvider`。
//! 第 07 课：压缩策略在 `repl` 里换一行；默认 `NoCompaction`。
//! 第 09 课：工具在 `tools::default_registry()` 登记，不要在这里列名单。
//! 单课小 demo 在 `examples/NN-*.rs`。`bash scripts/run.sh` 跑这里。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    // 总体 = 已完成课的合集。live 接 DeepSeek；循环只认 Provider。
    run_repl(true);
}
