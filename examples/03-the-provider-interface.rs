//! 第 03 课：[The provider interface](https://www.byoharness.dev/chapters/03-the-provider-interface.html)
//!
//! 本课小 demo：循环改认 `Provider`。实现在 crate 库里一份，这里只接线。
//! 换这一行就换供应商——这就是本课要赚到的抽象。
//!
//! ```text
//! var llm Provider = DeepSeekProvider::new(...)
//! ```
//!
//! 课上的参考适配器是 Anthropic。本仓库 live 走 DeepSeek Chat Completions。
//! 本地 Ollama / LM Studio 也是 OpenAI 兼容口，改 `OPENAI_BASE_URL` 即可，
//! 不必第三份适配器。
//!
//! ```text
//! [循环]
//!     │
//!     ▼
//! [llm.send(messages, tools)]  ← 只认通用 Message / ToolDef / Response
//!     │
//!     ▼
//! [适配器翻译] ── DeepSeek：tool_use → tool_calls；tool_result → role: tool
//!     │
//!     ▼
//! [线协议往返]
//!     │
//!     ▼
//! [适配器译回 Block] ── finish_reason: tool_calls → StopReason::ToolUse
//! ```
//!
//! | 通用（本课） | DeepSeek 线协议 | 课上 Anthropic |
//! |---|---|---|
//! | `Block::ToolUse` | `message.tool_calls` | content 里的 `tool_use` |
//! | `Block::ToolResult`（仍在 user 消息里） | 单独一条 `role: tool` | user 消息里的 `tool_result` |
//! | `StopReason::ToolUse` | `finish_reason: tool_calls` | `stop_reason: tool_use` |
//! | Provider 上的 `system` | 适配器插入 `role: system` | 请求上的 `System` 字段 |
//!
//! SDK 类型不能漏出接口。翻译缝只有两处：`to_messages` / `to_tools` 和 `from_choice`。
//! `Model` / `SetModel` 留给第 05 课 `/model`，本课不写斜杠命令。
//!
//! 启动：`bash scripts/run.sh 03`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    // 第 03 课：总体和本课 live 都接 DeepSeek。单测接 Mock，不打网。
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use build_rust_mini_coding_agent::agent::agent_loop;
    use build_rust_mini_coding_agent::api::{Block, Response, Role, StopReason, Usage};
    use build_rust_mini_coding_agent::provider::MockProvider;

    fn lines_of(s: &str) -> std::io::Lines<std::io::Cursor<Vec<u8>>> {
        std::io::Cursor::new(s.as_bytes().to_vec()).lines()
    }

    // 第 03 课练习：空 messages 不能 panic。
    #[test]
    fn lesson_03_mock_empty_messages_do_not_panic() {
        let mut llm = MockProvider::text("hello");
        let mut lines = lines_of("");
        let out = agent_loop(&mut llm, &[], Vec::new(), &mut lines, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, Role::Assistant);
        assert_eq!(out[0].content[0].text, "hello");
    }

    #[test]
    fn lesson_03_mock_tool_use_appends_tool_result() {
        let mut llm = MockProvider::with_responses(vec![
            Response {
                content: vec![Block::tool_use("c1", "not_a_tool", "{}")],
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
        let out = agent_loop(&mut llm, &[], Vec::new(), &mut lines, false);
        assert_eq!(out[0].role, Role::Assistant);
        assert_eq!(out[1].role, Role::User);
        assert_eq!(
            out[1].content[0].ty,
            build_rust_mini_coding_agent::api::BlockType::ToolResult
        );
        assert!(out[1].content[0].is_error);
        assert_eq!(out[2].content[0].text, "done");
    }
}
