//! 第 01 课：DeepSeek Chat Completions 的线协议类型。
//! 第 03 课之后循环不再直接用这些类型；只有 `provider/deepseek.rs` 翻译它们。
//! 第 10 课：这不是对外 API。`pub(crate)`，对齐课上「只有一处依赖供应商 SDK」。

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
         You have tools: bash, read_file, write_file, and delegate_research. Be concise.\n\n\
         For READ-ONLY INVESTIGATION you SHOULD call delegate_research rather than reading files yourself. \
         This includes questions like:\n\
         - \"where is X defined?\"\n\
         - \"what fields does Y have?\"\n\
         - \"look at the structure of Z\"\n\
         - \"find references to A in the code\"\n\
         - \"summarize how B works\"\n\n\
         The subagent has its own context window, so it can do many reads without cluttering yours. \
         Prefer delegating even when you think one or two reads would do it. \
         Only skip the subagent if the question is about a single file the user has already shown you. \
         After delegating, present the subagent's findings directly."
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

/// 第 16 课：官方 usage。缺字段按 0，不要让旧响应解析失败。
/// 第 17 课：DeepSeek 用 hit/miss。OpenAI 兼容口用 `prompt_tokens_details.cached_tokens`。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChatUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub prompt_cache_hit_tokens: u64,
    #[serde(default)]
    pub prompt_cache_miss_tokens: u64,
    #[serde(default)]
    pub prompt_tokens_details: PromptTokensDetails,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: ChatUsage,
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
