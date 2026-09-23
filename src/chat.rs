//! 第 01 课：DeepSeek Chat Completions 的线协议类型。
//! 第 03 课之后循环不再直接用这些类型；只有 `provider/deepseek.rs` 翻译它们。

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_MODEL: &str = "deepseek-flash";
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

// 第 01 课：只封输出。输入历史是第 06 / 07 课的事。
pub const MAX_TOKENS: u32 = 8192;

// 第 01 课：模型看不见请求里的 `model=`。身份必须写进 system。
pub fn system_prompt(model: &str) -> String {
    format!(
        "You are DeepSeek, a coding assistant running in a terminal. \
         The model serving this session is {model} (DeepSeek, not Claude, not GPT). \
         You have three tools: bash, read_file, write_file. Be concise."
    )
}

/// 第 01 课：一条对话消息。API 无状态，客户端带着全部历史（第 06 课展开）。
/// 第 03 课：这是线协议，不是循环用的 `api::Message`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub finish_reason: Option<String>,
    pub message: ChatMessage,
}

/// DeepSeek 官方与 dsh 都是 `{base}/chat/completions`，不要默认再拼 `/v1`。
pub fn chat_completions_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

/// 错误正文里可能回显密钥片段。只留 type/code。
pub fn redact_api_error(raw: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        let err = &value["error"];
        if err.is_object() {
            return format!(
                "type={} code={}",
                err["type"].as_str().unwrap_or("?"),
                err["code"].as_str().unwrap_or("?")
            );
        }
    }
    if raw.len() > 160 {
        format!("{}…", raw.chars().take(160).collect::<String>())
    } else {
        raw.to_string()
    }
}
