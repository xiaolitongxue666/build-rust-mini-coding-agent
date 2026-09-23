//! 第 06 课：[Conversation state](https://www.byoharness.dev/chapters/06-conversation-state.html)
//!
//! 本课小 demo：把「模型没有记忆」写清楚。实现仍在 crate 库一份，这里只接线。
//!
//! ```text
//! POST /chat/completions → response
//! POST /chat/completions → response
//! POST /chat/completions → response
//! ```
//!
//! 三次调用彼此独立。没有 session。对话能续上，是因为客户端每次把整段
//! `messages` 再寄出去。
//!
//! | 步骤 | 往切片里加什么 | 为什么 |
//! |---|---|---|
//! | 你提交一行 | `{Role: User, Content: [Text]}` | 你的话 |
//! | 模型回复 | `{Role: Assistant, Content: resp}` | 含任何 `tool_use` |
//! | 工具跑完 | `{Role: User, Content: [ToolResult, ...]}` | 本轮全部结果，id 必须对上 |
//! | 循环再 `send` | 不加新条目 | 循环在切片上转，不是新开一段 |
//!
//! 一轮用了一个工具之后，切片是四条：user 文本 → assistant（tool_use）→
//! user（tool_result）→ assistant 终稿。system 不在这四条里。
//!
//! `/clear` 是一行 `messages.clear()`：模型无状态，清本地切片就是清记忆。
//!
//! 本课不写压缩、不写 prompt cache。那是后面的课。
//!
//! 启动：`bash scripts/run.sh 06`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use build_rust_mini_coding_agent::agent::agent_loop;
    use build_rust_mini_coding_agent::api::{Block, Message, Response, Role, StopReason, Usage};
    use build_rust_mini_coding_agent::commands::clear_conversation;
    use build_rust_mini_coding_agent::conversation::{
        messages_json, orphaned_tool_result_ids, tool_pairs_intact,
    };
    use build_rust_mini_coding_agent::provider::MockProvider;

    fn lines_of(s: &str) -> std::io::Lines<std::io::Cursor<Vec<u8>>> {
        std::io::Cursor::new(s.as_bytes().to_vec()).lines()
    }

    fn one_tool_turn() -> (MockProvider, Vec<Message>) {
        let mut llm = MockProvider::with_responses(vec![
            Response {
                content: vec![Block::tool_use("toolu_01abc", "not_a_tool", "{}")],
                stop_reason: StopReason::ToolUse,
                usage: Usage::default(),
            },
            Response {
                content: vec![Block::text("done")],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            },
        ]);
        let mut lines = lines_of("");
        let start = vec![Message::user_text("list files")];
        let out = agent_loop(&mut llm, &[], start, &mut lines, false);
        (llm, out)
    }

    #[test]
    fn lesson_06_one_tool_turn_has_four_entries() {
        let (llm, out) = one_tool_turn();
        assert_eq!(out.len(), 4);
        assert_eq!(out[0].role, Role::User);
        assert_eq!(out[1].role, Role::Assistant);
        assert_eq!(out[1].content[0].tool_use_id, "toolu_01abc");
        assert_eq!(out[2].role, Role::User);
        assert_eq!(out[2].content[0].tool_use_id, "toolu_01abc");
        assert_eq!(out[3].content[0].text, "done");
        assert!(tool_pairs_intact(&out));

        let snapshots = llm.sent();
        assert_eq!(snapshots[0].len(), 1);
        assert_eq!(snapshots[1].len(), 3);
    }

    #[test]
    fn lesson_06_dump_shows_pairing() {
        let (_llm, out) = one_tool_turn();
        let dump = messages_json(&out);
        let rows: serde_json::Value = serde_json::from_str(&dump).expect("json");
        assert_eq!(rows.as_array().map(|a| a.len()), Some(4));
        assert_eq!(rows[0]["role"], "user");
        assert_eq!(rows[1]["content"][0]["type"], "tool_use");
        assert_eq!(rows[1]["content"][0]["tool_use_id"], "toolu_01abc");
        assert_eq!(rows[2]["content"][0]["type"], "tool_result");
        assert_eq!(rows[2]["content"][0]["tool_use_id"], "toolu_01abc");
    }

    #[test]
    fn lesson_06_dropping_assistant_orphans_tool_result() {
        let (_llm, mut out) = one_tool_turn();
        out.remove(1);
        assert_eq!(
            orphaned_tool_result_ids(&out),
            vec!["toolu_01abc".to_string()]
        );
        assert!(!tool_pairs_intact(&out));
    }

    #[test]
    fn lesson_06_clear_empties_slice() {
        let (_llm, mut out) = one_tool_turn();
        clear_conversation(&mut out);
        assert!(out.is_empty());
        assert_eq!(messages_json(&out), "[]");
    }
}
