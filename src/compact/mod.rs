//! 第 07 课：[Compaction strategies](https://www.byoharness.dev/chapters/07-compaction-strategies.html)
//!
//! 第一次必须扔掉信息。策略和 Provider 一样可换：一个接口、多份实现，
//! REPL 里换一行。
//!
//! 压缩只在 `agent_loop` 每轮开头调用，**不要**放进 `Provider::send`。
//! Summarize 会再调一次 `send`；套进去会无限递归。
//!
//! 本课不写 TokenBudget（课上练习）。
//! 第 17 课：压缩只改 `messages`，不改 system。DeepSeek 的磁盘缓存认完整前缀单元；
//! 砍掉开头的历史，旧单元就对不上。system 不在这个切片里。
//! 第 10 课：对应课上 `internal/compact/`。策略文件同目录，不拆子 crate。

mod logging;
mod nocompaction;
mod sliding;
mod summarize;

pub use logging::{with_logging, LoggingStrategy};
pub use nocompaction::NoCompaction;
pub use sliding::SlidingWindow;
pub use summarize::Summarize;

use crate::api::{Message, Role};
use crate::provider::Provider;

/// 第 07 课：多数时候原样返回；越过阈值才缩短切片。
pub trait CompactionStrategy {
    fn compact(&self, messages: &[Message], llm: &dyn Provider) -> Result<Vec<Message>, String>;
}

/// 从 `desired` 往回走到干净边界：带文本的 user，且不是 tool_result。
/// 找不到就回 0（什么都不做），好过切开一对 tool_use / tool_result。
pub fn safe_split_point(messages: &[Message], desired: usize) -> usize {
    if desired == 0 {
        return 0;
    }
    if desired >= messages.len() {
        return messages.len();
    }
    for i in (1..=desired).rev() {
        if messages[i].role == Role::User && !messages[i].has_tool_result() {
            return i;
        }
    }
    0
}

pub fn print_compaction(before: &[Message], after: &[Message]) {
    println!("--- compaction: {} → {} ---", before.len(), after.len());
    print!("{}", crate::api::render_transcript(before));
    println!("---");
    print!("{}", crate::api::render_transcript(after));
    println!("---");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Block, Message, Role};
    use crate::provider::MockProvider;

    fn text_user(text: &str) -> Message {
        Message::user_text(text)
    }

    fn text_assistant(text: &str) -> Message {
        Message::assistant(vec![Block::text(text)])
    }

    fn tool_use(id: &str, name: &str) -> Message {
        Message::assistant(vec![Block::tool_use(id, name, "{}")])
    }

    fn tool_result(id: &str, result: &str) -> Message {
        Message::tool_results(vec![Block::tool_result(id, result, false)])
    }

    #[test]
    fn safe_split_at_clean_boundary() {
        let msgs = vec![
            text_user("hi"),
            text_assistant("hello"),
            text_user("how are you"),
        ];
        assert_eq!(safe_split_point(&msgs, 2), 2);
    }

    #[test]
    fn safe_split_walks_back_over_tool_pair() {
        let msgs = vec![
            text_user("q1"),
            text_assistant("a1"),
            text_user("q2"),
            tool_use("t1", "echo"),
            tool_result("t1", "ok"),
            text_assistant("done"),
        ];
        assert_eq!(safe_split_point(&msgs, 4), 2);
    }

    #[test]
    fn safe_split_gives_up_at_zero() {
        let msgs = vec![tool_result("t1", "ok"), text_assistant("....")];
        assert_eq!(safe_split_point(&msgs, 1), 0);
    }

    #[test]
    fn safe_split_edges() {
        let msgs = vec![text_user("x")];
        assert_eq!(safe_split_point(&msgs, 0), 0);
        assert_eq!(safe_split_point(&msgs, 5), 1);
    }

    #[test]
    fn no_compaction_is_passthrough() {
        let msgs = vec![text_user("a"), text_assistant("b")];
        let llm = MockProvider::text("x");
        let out = NoCompaction.compact(&msgs, &llm).expect("ok");
        assert_eq!(out, msgs);
    }

    #[test]
    fn sliding_window_below_threshold_is_noop() {
        let msgs = vec![text_user("a"), text_assistant("b")];
        let llm = MockProvider::text("x");
        let out = SlidingWindow { keep_last: 10 }
            .compact(&msgs, &llm)
            .expect("ok");
        assert_eq!(out.len(), msgs.len());
    }

    #[test]
    fn sliding_window_drops_prefix() {
        let msgs = vec![
            text_user("1"),
            text_assistant("2"),
            text_user("3"),
            text_assistant("4"),
            text_user("5"),
        ];
        let llm = MockProvider::text("x");
        let out = SlidingWindow { keep_last: 2 }
            .compact(&msgs, &llm)
            .expect("ok");
        assert!(out.len() < msgs.len());
        assert!(!out[0].has_tool_result());
        assert_eq!(out[0].role, Role::User);
    }

    #[test]
    fn sliding_window_never_splits_tool_pair() {
        let msgs = vec![
            text_user("q1"),
            text_assistant("a1"),
            text_user("q2"),
            tool_use("t1", "echo"),
            tool_result("t1", "ok"),
            text_assistant("done"),
            text_user("q3"),
        ];
        let llm = MockProvider::text("x");
        let out = SlidingWindow { keep_last: 3 }
            .compact(&msgs, &llm)
            .expect("ok");
        for message in &out {
            for block in &message.content {
                if block.ty == crate::api::BlockType::ToolResult {
                    let paired = out.iter().any(|m| {
                        m.content.iter().any(|b| {
                            b.ty == crate::api::BlockType::ToolUse
                                && b.tool_use_id == block.tool_use_id
                        })
                    });
                    assert!(paired, "orphaned {}", block.tool_use_id);
                }
            }
        }
    }

    #[test]
    fn summarize_below_threshold_is_noop() {
        let msgs = vec![text_user("a"), text_assistant("b")];
        let llm = MockProvider::text("summary");
        let out = Summarize {
            threshold: 20,
            keep_recent: 2,
            instructions: String::new(),
        }
        .compact(&msgs, &llm)
        .expect("ok");
        assert_eq!(out, msgs);
        assert!(llm.sent().is_empty());
    }

    #[test]
    fn summarize_replaces_old_half() {
        let msgs = vec![
            text_user("q1"),
            text_assistant("a1"),
            text_user("q2"),
            text_assistant("a2"),
            text_user("q3"),
        ];
        let llm = MockProvider::text("kept the path src/main.rs");
        let out = Summarize {
            threshold: 0,
            keep_recent: 2,
            instructions: String::new(),
        }
        .compact(&msgs, &llm)
        .expect("ok");
        assert!(out[0].content[0]
            .text
            .starts_with("[earlier conversation summary]"));
        assert!(out[0].content[0].text.contains("src/main.rs"));
        assert_eq!(out[1].content[0].text, "q2");
        assert_eq!(llm.sent().len(), 1);
        assert_eq!(llm.sent()[0].len(), 1);
    }
}
