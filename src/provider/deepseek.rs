//! 第 03 课：DeepSeek 适配器。这是 harness 里**唯一**知道 Chat Completions 词表的地方。
//!
//! 翻译缝只有两处，和课上 `anthropic.go` 对齐：
//! - `to_messages` / `to_tools`：通用类型 → 线协议
//! - `from_choice`：线协议 → 通用 `Response`
//!
//! Anthropic 的 system 是请求上的 `System` 字段。DeepSeek / OpenAI 没有那一栏，
//! 适配器在这里插第一条 `role: system`。循环看不见这个差异。
//! 第 06 课：system 每次请求另带，不进通用 `messages` 切片。

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use serde_json::{json, Value};

use crate::api::{Block, Message, Response, Role, StopReason, ToolDef};
use crate::chat::{
    chat_completions_url, redact_api_error, system_prompt, ChatMessage, ChatResponse, Choice,
    FunctionCall, ToolCall, DEFAULT_BASE_URL, DEFAULT_MODEL, MAX_TOKENS,
};
use crate::provider::Provider;

#[derive(Clone)]
pub struct DeepSeekProvider {
    http: Client,
    api_key: String,
    endpoint: String,
    model: String,
    system: String,
    max_tokens: u32,
}

impl DeepSeekProvider {
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let model = model.into();
        let endpoint = chat_completions_url(&base_url.into());
        let http = Client::builder()
            .redirect(Policy::none())
            .build()
            .expect("reqwest client");
        Self {
            http,
            api_key: api_key.into(),
            endpoint,
            system: system_prompt(&model),
            model,
            max_tokens: MAX_TOKENS,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return Err("缺少 DEEPSEEK_API_KEY。请用 bash scripts/run.sh 启动。".to_string());
        }
        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        let base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Ok(Self::new(api_key, model, base_url))
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn api_key_len(&self) -> usize {
        self.api_key.len()
    }

    /// 通用 → Chat Completions。`tool_use` 变 `tool_calls`；`tool_result` 变成单独的 `role: tool`。
    pub(crate) fn to_messages(&self, messages: &[Message]) -> Vec<ChatMessage> {
        to_messages(&self.system, messages)
    }

    pub(crate) fn to_tools(&self, tools: &[ToolDef]) -> Value {
        to_tools(tools)
    }
}

impl Provider for DeepSeekProvider {
    fn send(&self, messages: &[Message], tools: &[ToolDef]) -> Result<Response, String> {
        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": self.to_messages(messages),
        });
        if !tools.is_empty() {
            body["tools"] = self.to_tools(tools);
        }

        let resp = self
            .http
            .post(&self.endpoint)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("{status} {}", redact_api_error(&text)));
        }
        let parsed: ChatResponse =
            serde_json::from_str(&text).map_err(|e| format!("{e}: {}", redact_api_error(&text)))?;
        let Some(choice) = parsed.choices.into_iter().next() else {
            return Err("empty choices".to_string());
        };
        Ok(from_choice(choice))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, name: String) {
        self.model = name;
        self.system = system_prompt(&self.model);
    }
}

pub(crate) fn to_messages(system: &str, messages: &[Message]) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    if !system.is_empty() {
        out.push(ChatMessage {
            role: "system".to_string(),
            content: Some(Value::String(system.to_string())),
            tool_calls: None,
            tool_call_id: None,
        });
    }
    for message in messages {
        match message.role {
            Role::User => append_user(&mut out, message),
            Role::Assistant => append_assistant(&mut out, message),
        }
    }
    out
}

fn append_user(out: &mut Vec<ChatMessage>, message: &Message) {
    let mut texts = Vec::new();
    for block in &message.content {
        match block.ty {
            crate::api::BlockType::ToolResult => out.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(Value::String(block.tool_result.clone())),
                tool_calls: None,
                tool_call_id: Some(block.tool_use_id.clone()),
            }),
            crate::api::BlockType::Text => {
                if !block.text.is_empty() {
                    texts.push(block.text.clone());
                }
            }
            crate::api::BlockType::ToolUse => {}
        }
    }
    if !texts.is_empty() {
        out.push(ChatMessage {
            role: "user".to_string(),
            content: Some(Value::String(texts.join("\n"))),
            tool_calls: None,
            tool_call_id: None,
        });
    }
}

fn append_assistant(out: &mut Vec<ChatMessage>, message: &Message) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    for block in &message.content {
        match block.ty {
            crate::api::BlockType::Text => {
                if !block.text.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&block.text);
                }
            }
            crate::api::BlockType::ToolUse => tool_calls.push(ToolCall {
                id: block.tool_use_id.clone(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: block.tool_name.clone(),
                    arguments: block.tool_input.clone(),
                },
            }),
            crate::api::BlockType::ToolResult => {}
        }
    }
    out.push(ChatMessage {
        role: "assistant".to_string(),
        content: if text.is_empty() {
            None
        } else {
            Some(Value::String(text))
        },
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        tool_call_id: None,
    });
}

pub(crate) fn to_tools(tools: &[ToolDef]) -> Value {
    // 第 03 课陷阱：map 迭代顺序随机，同一套工具可能编出不同字节，第 06 课 cache 会失效。
    // 第 09 课 Registry 再按名字排序。这里按调用方给的切片顺序发。
    Value::Array(
        tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": {
                            "type": "object",
                            "properties": tool.input_schema,
                            "required": tool.required,
                        }
                    }
                })
            })
            .collect(),
    )
}

pub(crate) fn from_choice(choice: Choice) -> Response {
    let finish = choice.finish_reason.unwrap_or_default();
    let stop_reason = from_stop_reason(&finish);

    let mut content = Vec::new();
    if let Some(Value::String(text)) = &choice.message.content {
        if !text.is_empty() {
            content.push(Block::text(text.clone()));
        }
    }
    for call in choice.message.tool_calls.unwrap_or_default() {
        content.push(Block::tool_use(
            call.id,
            call.function.name,
            call.function.arguments,
        ));
    }
    Response {
        content,
        stop_reason,
    }
}

pub(crate) fn from_stop_reason(finish: &str) -> StopReason {
    match finish {
        "tool_calls" => StopReason::ToolUse,
        "stop" => StopReason::EndTurn,
        _ => StopReason::Other,
    }
}
