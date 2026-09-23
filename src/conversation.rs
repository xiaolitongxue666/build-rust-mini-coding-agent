//! 第 06 课：[Conversation state](https://www.byoharness.dev/chapters/06-conversation-state.html)
//!
//! 模型没有记忆。每次 `Provider::send` 都把整段 `messages` 重发；没有 session id。
//! 真相来源就是这块切片。system 不在切片里，挂在具体 Provider 上，每次请求另带。
//!
//! 不要在这里做向量库或检索。课上说整段切片对 coding agent 够用；
//! 变长以后的压缩在 `compact` 模块（第 07 课）。

use serde_json::{json, Value};

use crate::api::{Block, BlockType, Message, Role};

/// 第 06 课练习：把切片打成 JSON，核对 user/assistant 交替和 tool_use / tool_result 配对。
pub fn messages_json(messages: &[Message]) -> String {
    let rows: Vec<Value> = messages.iter().map(message_value).collect();
    serde_json::to_string_pretty(&rows).expect("message dump is valid json")
}

fn message_value(message: &Message) -> Value {
    json!({
        "role": match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        },
        "content": message.content.iter().map(block_value).collect::<Vec<_>>(),
    })
}

fn block_value(block: &Block) -> Value {
    match block.ty {
        BlockType::Text => json!({
            "type": "text",
            "text": block.text,
        }),
        BlockType::ToolUse => json!({
            "type": "tool_use",
            "tool_use_id": block.tool_use_id,
            "tool_name": block.tool_name,
            "tool_input": block.tool_input,
        }),
        BlockType::ToolResult => json!({
            "type": "tool_result",
            "tool_use_id": block.tool_use_id,
            "tool_result": block.tool_result,
            "is_error": block.is_error,
        }),
    }
}

/// 第 06 课：每条 `tool_result` 必须引用前面出现过的同一个 `tool_use_id`。
/// 对不上，真实 API 会 400（孤立的 tool_result）。
pub fn orphaned_tool_result_ids(messages: &[Message]) -> Vec<String> {
    let mut seen_use = std::collections::HashSet::new();
    let mut orphans = Vec::new();
    for message in messages {
        for block in &message.content {
            match block.ty {
                BlockType::ToolUse => {
                    seen_use.insert(block.tool_use_id.clone());
                }
                BlockType::ToolResult => {
                    if !seen_use.contains(&block.tool_use_id) {
                        orphans.push(block.tool_use_id.clone());
                    }
                }
                BlockType::Text => {}
            }
        }
    }
    orphans
}

pub fn tool_pairs_intact(messages: &[Message]) -> bool {
    orphaned_tool_result_ids(messages).is_empty()
}
