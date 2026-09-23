//! 总体入口：已完成课接到一起（01–10）。接线层，对齐课上 `main.go`。
//! 第 10 课：不要在这里堆领域文件；领域在库模块里。
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
