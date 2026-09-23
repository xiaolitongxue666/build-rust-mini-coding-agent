//! 第 03 课：[The provider interface](https://www.byoharness.dev/chapters/03-the-provider-interface.html)
//! 第 10 课：本模块在依赖栈最底，不 `use crate::` 其它模块。
//!
//! 循环、工具、门只认这套类型。线协议（Anthropic Messages 或 DeepSeek Chat Completions）
//! 不得漏到 `Provider` 以外。课上的类型是各家 API 的**交集**，不是某一家的 SDK。
//!
//! 不带 `Role::System`：第 06 课写明 system 不在 `messages` 里，挂在具体 Provider 上
//!（Anthropic 的 `System` 字段，DeepSeek 在适配器里插成第一条 `role: system`）。
//! 第 16 课：`Usage` 是各家「token」的交集。DeepSeek 填 cache miss / hit / output，cache 写入保持 0。

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Text,
    ToolUse,
    ToolResult,
}

/// 一条内容。字段按 `ty` 解读，多余字段忽略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub ty: BlockType,
    pub text: String,
    pub tool_use_id: String,
    pub tool_name: String,
    /// 第 03 课：`tool_use` 的入参是原始 JSON，适配器原样交给供应商。
    pub tool_input: String,
    pub tool_result: String,
    pub is_error: bool,
}

impl Block {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            ty: BlockType::Text,
            text: text.into(),
            tool_use_id: String::new(),
            tool_name: String::new(),
            tool_input: String::new(),
            tool_result: String::new(),
            is_error: false,
        }
    }

    pub fn tool_use(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        tool_input: impl Into<String>,
    ) -> Self {
        Self {
            ty: BlockType::ToolUse,
            text: String::new(),
            tool_use_id: tool_use_id.into(),
            tool_name: tool_name.into(),
            tool_input: tool_input.into(),
            tool_result: String::new(),
            is_error: false,
        }
    }

    pub fn tool_result(
        tool_use_id: impl Into<String>,
        tool_result: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            ty: BlockType::ToolResult,
            text: String::new(),
            tool_use_id: tool_use_id.into(),
            tool_name: String::new(),
            tool_input: String::new(),
            tool_result: tool_result.into(),
            is_error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Block>,
}

impl Message {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![Block::text(text)],
        }
    }

    pub fn assistant(content: Vec<Block>) -> Self {
        Self {
            role: Role::Assistant,
            content,
        }
    }

    /// 第 03 课：通用形状里 tool_result 仍是 user 消息里的块。
    /// DeepSeek 适配器再拆成 `role: tool`。
    pub fn tool_results(content: Vec<Block>) -> Self {
        Self {
            role: Role::User,
            content,
        }
    }

    /// 第 07 课：干净切分点是「带文本的 user」，不是 tool_result 回包。
    pub fn has_tool_result(&self) -> bool {
        self.content
            .iter()
            .any(|block| block.ty == BlockType::ToolResult)
    }
}

/// 第 07 课：摘要提示和压缩日志用的可读转写。
pub fn render_transcript(messages: &[Message]) -> String {
    let mut out = String::new();
    for message in messages {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        out.push_str(role);
        out.push_str(": ");
        for block in &message.content {
            match block.ty {
                BlockType::Text => out.push_str(&block.text),
                BlockType::ToolUse => {
                    out.push_str(&format!(
                        "[called {} with {}]",
                        block.tool_name, block.tool_input
                    ));
                }
                BlockType::ToolResult => {
                    out.push_str(&format!("[tool result: {}]", block.tool_result));
                }
            }
            out.push('\n');
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema 的 `properties`。第 09 课 `Registry::definitions` 按名字排序，避免打乱 cache。
    pub input_schema: Map<String, Value>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    Other,
}

/// 第 16 课：一次响应的用量。任何有 token 概念的后端都能填；没有就留 0。
/// `input_tokens` 是 cache miss。`cache_read_tokens` 是 cache hit。
/// DeepSeek 没有 Anthropic 那种 cache 写入价，`cache_creation_tokens` 保持 0。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

impl Usage {
    pub fn plus(self, other: Usage) -> Usage {
        Usage {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            cache_creation_tokens: self.cache_creation_tokens + other.cache_creation_tokens,
            cache_read_tokens: self.cache_read_tokens + other.cache_read_tokens,
        }
    }

    pub fn is_zero(self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.cache_creation_tokens == 0
            && self.cache_read_tokens == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub content: Vec<Block>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}
