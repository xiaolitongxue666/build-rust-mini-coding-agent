//! 第 01 课内层循环。第 02 课用 `use_gate` 在执行前插入审批，两个出口不变。
//! 第 03 课：循环只认 `Provider` + 通用 `Message`，不再直接打 Chat Completions。

use std::io;

use crate::api::{Block, Message, StopReason, ToolDef};
use crate::gate::{execute_direct, execute_gated};
use crate::provider::Provider;

/// `use_gate`：总体和第 02 课为 true；第 01 课 demo 为 false。
pub fn agent_loop(
    llm: &mut dyn Provider,
    tools: &[ToolDef],
    mut messages: Vec<Message>,
    lines: &mut impl Iterator<Item = io::Result<String>>,
    use_gate: bool,
) -> Vec<Message> {
    loop {
        let resp = match llm.send(&messages, tools) {
            Ok(r) => r,
            Err(err) => {
                println!("api error: {err}");
                return messages;
            }
        };

        for block in &resp.content {
            if block.ty == crate::api::BlockType::Text && !block.text.is_empty() {
                println!("{}", block.text);
            }
        }

        // 第 01 课陷阱：必须把 assistant 原样 append 回去。现在是通用 Block，不是线协议。
        let tool_uses: Vec<Block> = resp
            .content
            .iter()
            .filter(|b| b.ty == crate::api::BlockType::ToolUse)
            .cloned()
            .collect();
        messages.push(Message::assistant(resp.content));

        if resp.stop_reason != StopReason::ToolUse || tool_uses.is_empty() {
            return messages;
        }

        let mut results = Vec::new();
        for call in tool_uses {
            let (result, is_err) = if use_gate {
                execute_gated(&call.tool_name, &call.tool_input, lines)
            } else {
                execute_direct(&call.tool_name, &call.tool_input)
            };
            if is_err {
                eprintln!("[tool error] {}", truncate(&result, 200));
            }
            results.push(Block::tool_result(call.tool_use_id, result, is_err));
        }
        messages.push(Message::tool_results(results));
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
