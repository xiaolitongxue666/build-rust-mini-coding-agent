//! 第 10 课：[Project structure](https://www.byoharness.dev/chapters/10-project-structure.html)
//!
//! 课上是 Go：扁平 `package main` 拆进 `internal/`。本仓库是 Rust，已经是
//! [Cargo 包布局](https://doc.rust-lang.org/cargo/guide/project-layout.html)：
//!
//! ```text
//! src/lib.rs      库（领域模块）
//! src/main.rs     bin（接线，对齐 main.go）
//! examples/       单课 demo
//! ```
//!
//! | 课上 Go `internal/` | 本仓库 |
//! |---|---|
//! | `internal/api` | `src/api.rs` |
//! | `internal/provider` | `src/provider/` |
//! | `internal/tool` | `src/tools/` |
//! | `internal/compact` | `src/compact/` |
//! | `internal/ui` | `src/ui/` |
//! | `main.go` + `commands.go` | `src/main.rs` + `src/repl.rs` + `src/commands.rs` |
//!
//! 不要再套 `src/internal/`：模块默认私有，`publish = false` 已经挡住外人。
//! 模块按[领域](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)分。
//! 线协议 `chat` 是 `pub(crate)`，只有适配器翻译。
//!
//! 本课不写子 agent、不写整屏 TUI、不拆 workspace。
//!
//! 启动：`bash scripts/run.sh 10`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::api::Message;
    use build_rust_mini_coding_agent::compact::{CompactionStrategy, NoCompaction};
    use build_rust_mini_coding_agent::provider::{MockProvider, Provider};
    use build_rust_mini_coding_agent::tools::{default_registry, Tool};

    fn assert_provider(_: &dyn Provider) {}
    fn assert_tool(_: &dyn Tool) {}
    fn assert_compact(_: &dyn CompactionStrategy) {}

    #[test]
    fn lesson_10_three_seams_are_modules() {
        let llm = MockProvider::text("x");
        assert_provider(&llm);
        let defs = default_registry().definitions();
        assert_eq!(defs[0].name, "bash");
        let compact = NoCompaction;
        assert_compact(&compact);
        let _ = Message::user_text("hi");
    }

    #[test]
    fn lesson_10_api_does_not_need_other_crate_modules() {
        let msg = Message::user_text("bottom");
        assert_eq!(msg.content[0].text, "bottom");
    }

    #[test]
    fn lesson_10_tools_register_outside_main() {
        let names: Vec<_> = default_registry()
            .definitions()
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert!(names.contains(&"read_file".to_string()));
        let (text, is_err) = default_registry().execute("missing", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
        let _ = assert_tool;
    }
}
