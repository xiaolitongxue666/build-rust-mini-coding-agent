//! 总体入口：已完成课接到一起（01 循环 + 02 权限门 + 03 Provider + 04 UI + 05 斜杠命令）。
//! 对齐 [byo-coding-agent](https://github.com/betta-tech/byo-coding-agent) 的 `main.go` 接线角色。
//! 第 03 课：这里换一行就能换供应商；默认 `DeepSeekProvider`。
//! 单课小 demo 在 `examples/NN-*.rs`。`bash scripts/run.sh` 跑这里。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    // 总体 = 已完成课的合集。live 接 DeepSeek；循环只认 Provider。
    run_repl(true);
}
