//! 第 07 课：[Compaction strategies](https://www.byoharness.dev/chapters/07-compaction-strategies.html)
//!
//! 本课小 demo：对话变长后第一次扔掉信息。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! [agent_loop 每轮开头]
//!     │
//!     ▼
//! [compactor.Compact(messages)]  ← 多数时候原样返回
//!     │
//!     ▼
//! [llm.send]  ← 压缩在外面，Summarize 再 send 一次也不会递归
//! ```
//!
//! | 策略 | 做什么 | 何时用 |
//! |---|---|---|
//! | `NoCompaction` | 原样返回 | 默认 |
//! | `SlidingWindow{KeepLast: N}` | 只留最后 N 条 | 便宜，丢旧上下文 |
//! | `Summarize{Threshold, KeepRecent}` | 让模型摘要旧半段 | 多一次 API |
//! | `WithLogging(inner, path)` | 装饰器，写 before/after | 对比策略 |
//!
//! 切分必须走 `safe_split_point`：从欲切点往回走到「带文本的 user」。
//! 否则会把 `tool_use` 和 `tool_result` 切开，下一轮 API 400。
//!
//! `/compact [sliding|summarize|none]`、`/verbose [on|off]` 当场试，不用重启。
//!
//! 换策略：`src/repl.rs` 里 `let compact = NoCompaction;` 那一行。
//!
//! 启动：`bash scripts/run.sh 07`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use build_rust_mini_coding_agent::agent::agent_loop_with;
    use build_rust_mini_coding_agent::api::{Block, Message};
    use build_rust_mini_coding_agent::commands::{run_command, CommandCtx, CommandOutcome};
    use build_rust_mini_coding_agent::compact::{
        with_logging, CompactionStrategy, NoCompaction, SlidingWindow,
    };
    use build_rust_mini_coding_agent::provider::MockProvider;
    use build_rust_mini_coding_agent::tools::default_tool_defs;

    fn lines_of(s: &str) -> std::io::Lines<std::io::Cursor<Vec<u8>>> {
        std::io::Cursor::new(s.as_bytes().to_vec()).lines()
    }

    fn long_chat() -> Vec<Message> {
        let mut out = Vec::new();
        for i in 1..=7 {
            out.push(Message::user_text(format!("u{i}")));
            if i < 7 {
                out.push(Message::assistant(vec![Block::text(format!("a{i}"))]));
            }
        }
        out
    }

    fn run(
        line: &str,
        llm: &mut MockProvider,
        messages: &mut Vec<Message>,
        verbose: &mut bool,
    ) -> Option<CommandOutcome> {
        let tools = default_tool_defs();
        let compact = NoCompaction;
        let mut ctx = CommandCtx {
            llm,
            messages,
            tools: &tools,
            compact: &compact,
            verbose,
            subagents: build_rust_mini_coding_agent::commands::no_subagents(),
        };
        run_command(line, &mut ctx)
    }

    #[test]
    fn lesson_07_compact_sliding_shortens() {
        let mut llm = MockProvider::text("x");
        let mut messages = long_chat();
        let mut verbose = false;
        assert_eq!(
            run("/compact sliding", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert!(messages.len() < 13);
        assert!(!messages[0].has_tool_result());
    }

    #[test]
    fn lesson_07_compact_none_is_baseline() {
        let mut llm = MockProvider::text("x");
        let mut messages = long_chat();
        let mut verbose = false;
        assert_eq!(
            run("/compact none", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert_eq!(messages.len(), 13);
    }

    #[test]
    fn lesson_07_compact_summarize_prefixes_summary() {
        let mut llm = MockProvider::text("kept src/lib.rs");
        let mut messages = long_chat();
        let mut verbose = false;
        assert_eq!(
            run("/compact summarize", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert!(messages[0].content[0]
            .text
            .starts_with("[earlier conversation summary]"));
        assert!(messages[0].content[0].text.contains("src/lib.rs"));
    }

    #[test]
    fn lesson_07_verbose_toggles() {
        let mut llm = MockProvider::text("x");
        let mut messages = Vec::new();
        let mut verbose = false;
        assert_eq!(
            run("/verbose on", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert!(verbose);
        assert_eq!(
            run("/verbose", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert!(!verbose);
    }

    #[test]
    fn lesson_07_loop_sends_compacted_slice() {
        let mut llm = MockProvider::text("ok");
        let start = long_chat();
        let compact = SlidingWindow { keep_last: 2 };
        let mut lines = lines_of("");
        let _ = agent_loop_with(&mut llm, &[], start, &mut lines, false, &compact, false);
        let first = &llm.sent()[0];
        assert!(first.len() < 13, "loop must compact before send");
        assert_eq!(first[0].content[0].text, "u6");
    }

    #[test]
    fn lesson_07_logging_writes_when_length_changes() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "byo-compact-{}.log",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let llm = MockProvider::text("x");
        let logged = with_logging(SlidingWindow { keep_last: 2 }, path.to_string_lossy());
        let after = logged.compact(&long_chat(), &llm).expect("ok");
        let body = std::fs::read_to_string(&path).expect("log");
        let _ = std::fs::remove_file(&path);
        assert!(after.len() < 13);
        assert!(body.contains("BEFORE"), "{body}");
        assert!(body.contains("AFTER"), "{body}");
    }

    #[test]
    fn lesson_07_unknown_strategy_is_handled() {
        let mut llm = MockProvider::text("x");
        let mut messages = long_chat();
        let mut verbose = false;
        assert_eq!(
            run("/compact bananas", &mut llm, &mut messages, &mut verbose),
            Some(CommandOutcome::Handled)
        );
        assert_eq!(messages.len(), 13);
    }
}
