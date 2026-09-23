//! 第 03 课：DeepSeek 适配器。这是 harness 里**唯一**知道 Chat Completions 词表的地方。
//! 第 10 课：对齐课上「只有一处依赖供应商 SDK」——`crate::chat` 只在这里 import。
//!
//! 翻译缝只有两处，和课上 `anthropic.go` 对齐：
//! - `to_messages` / `to_tools`：通用类型 → 线协议
//! - `from_choice`：线协议 → 通用 `Response`
//!
//! Anthropic 的 system 是请求上的 `System` 字段。DeepSeek / OpenAI 没有那一栏，
//! 适配器在这里插第一条 `role: system`。循环看不见这个差异。
//! 第 06 课：system 每次请求另带，不进通用 `messages` 切片。
//! 第 15 课：`behavior` 是 harness 行为，`project_context` 是 AGENTS.md。`system` 是拼好的那一段。
//! `/model` 和子 agent 的 `set_system` 只换行为半边，上下文留着。
//! 第 16 课：usage 在这次响应回来时按北京时间入账。账本是 `Arc`，子 agent 的 clone 累进同一份。

use std::sync::{Arc, Mutex};

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use serde_json::{json, Value};

use crate::api::{Block, Message, Response, Role, StopReason, ToolDef, Usage};
use crate::chat::{
    chat_completions_url, redact_api_error, system_prompt, ChatMessage, ChatResponse, ChatUsage,
    Choice, FunctionCall, ToolCall, DEFAULT_BASE_URL, DEFAULT_MODEL, MAX_TOKENS,
};
use crate::pricing::{self, Beijing, Ledger, TokenReport};
use crate::provider::Provider;

#[derive(Clone)]
pub struct DeepSeekProvider {
    http: Client,
    api_key: String,
    endpoint: String,
    model: String,
    /// 第 15 课：harness 行为。`set_system` / `set_model` 只改这一半。
    behavior: String,
    /// 第 15 课：cwd 的 AGENTS.md。空串表示没读到文件。
    project_context: String,
    /// 行为 + 项目上下文。请求里插的是这一段。
    system: String,
    /// 第 16 课：子 agent clone 共享同一份累计。
    ledger: Arc<Mutex<Ledger>>,
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
        let behavior = system_prompt(&model);
        Self {
            http,
            api_key: api_key.into(),
            endpoint,
            model,
            behavior: behavior.clone(),
            project_context: String::new(),
            system: behavior,
            ledger: Arc::new(Mutex::new(Ledger::default())),
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

    /// 第 15 课：只换项目上下文，不把已经拼好的 system 再贴一遍。
    pub fn attach_project_context(&mut self, context: String) {
        self.project_context = context;
        self.fuse();
    }

    pub fn usage_ledger(&self) -> Arc<Mutex<Ledger>> {
        Arc::clone(&self.ledger)
    }

    /// 第 16 课：测试和离线示例按给定的北京时间入账，不打网络。
    pub fn record_usage_at(&self, usage: Usage, when: Beijing) {
        let mut ledger = self.ledger.lock().unwrap_or_else(|err| err.into_inner());
        ledger.add(&self.model, usage, when);
    }

    fn fuse(&mut self) {
        let mut fused = self.behavior.clone();
        fused.push_str(&self.project_context);
        self.system = fused;
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
        // 第 17 课：DeepSeek 磁盘缓存默认开启，请求里不要加 Anthropic 的 cache_control。
        // https://api-docs.deepseek.com/guides/kv_cache
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
        let usage = usage_from_chat(&parsed.usage);
        self.record_usage_at(usage, pricing::beijing_now());
        let Some(choice) = parsed.choices.into_iter().next() else {
            return Err("empty choices".to_string());
        };
        Ok(from_choice(choice, usage))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, name: String) {
        self.model = name;
        self.behavior = system_prompt(&self.model);
        self.fuse();
    }

    fn system(&self) -> &str {
        &self.system
    }

    fn set_system(&mut self, prompt: String) {
        self.behavior = prompt;
        self.fuse();
    }

    fn token_report(&self) -> Option<TokenReport> {
        let ledger = self.ledger.lock().unwrap_or_else(|err| err.into_inner());
        Some(ledger.report())
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
    // 第 03 课陷阱：map 迭代顺序随机，同一套工具可能编出不同字节。
    // 第 09 / 17 课：按名字排序后再发。调用方把顺序打乱，线上的 JSON 仍然一样。
    let mut ordered: Vec<&ToolDef> = tools.iter().collect();
    ordered.sort_by(|left, right| left.name.cmp(&right.name));
    Value::Array(
        ordered
            .into_iter()
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

/// DeepSeek 的 hit/miss 优先。两栏都是 0 时，再看 OpenAI 的 `cached_tokens`。
/// 仍然没有命中信息，整段 prompt 按 miss。不要把 hit 再加进 prompt。
/// DeepSeek 不另收 Anthropic 那种写入加价，`cache_creation_tokens` 保持 0。
pub(crate) fn usage_from_chat(raw: &ChatUsage) -> Usage {
    let (miss, hit) = if raw.prompt_cache_hit_tokens > 0 || raw.prompt_cache_miss_tokens > 0 {
        (raw.prompt_cache_miss_tokens, raw.prompt_cache_hit_tokens)
    } else {
        let hit = raw
            .prompt_tokens_details
            .cached_tokens
            .min(raw.prompt_tokens);
        (raw.prompt_tokens - hit, hit)
    };
    Usage {
        input_tokens: miss,
        output_tokens: raw.completion_tokens,
        cache_creation_tokens: 0,
        cache_read_tokens: hit,
    }
}

pub(crate) fn from_choice(choice: Choice, usage: Usage) -> Response {
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
        usage,
    }
}

pub(crate) fn from_stop_reason(finish: &str) -> StopReason {
    match finish {
        "tool_calls" => StopReason::ToolUse,
        "stop" => StopReason::EndTurn,
        _ => StopReason::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(model: &str) -> DeepSeekProvider {
        DeepSeekProvider::new("k", model, "https://api.deepseek.com")
    }

    #[test]
    fn project_context_survives_model_and_research_system() {
        let mut root = provider("deepseek-flash");
        let behavior = root.system().to_string();
        root.attach_project_context(
            "\n\n# Project context (from AGENTS.md)\n\nuse tabs\n".to_string(),
        );
        root.set_system(behavior);
        assert_eq!(root.system().matches("# Project context").count(), 1);
        assert!(root.system().contains("use tabs"));
        assert!(root.system().contains("deepseek-flash"));

        let mut research = root.clone();
        research.set_system("research only".to_string());
        assert!(research.system().starts_with("research only"));
        assert!(research.system().contains("use tabs"));
        assert!(root.system().contains("You are DeepSeek"));

        root.set_model("deepseek-chat".to_string());
        assert!(root.system().contains("use tabs"));
        assert!(root.system().contains("deepseek-chat"));
        assert!(!root.system().contains("deepseek-flash"));
    }

    #[test]
    fn clones_share_the_ledger() {
        let root = provider("deepseek-flash");
        let child = root.clone();
        let off = Beijing {
            year: 2026,
            month: 9,
            day: 21,
            hour: 20,
        };
        let usage = Usage {
            input_tokens: 1_000_000,
            ..Usage::default()
        };
        root.record_usage_at(usage, off);
        child.record_usage_at(usage, off);
        let report = root.token_report().unwrap();
        assert!((report.yuan - 2.0).abs() < 1e-9);
    }

    #[test]
    fn prompt_without_cache_fields_is_a_miss() {
        let usage = usage_from_chat(&ChatUsage {
            prompt_tokens: 30,
            completion_tokens: 4,
            ..ChatUsage::default()
        });
        assert_eq!(usage.input_tokens, 30);
        assert_eq!(usage.cache_read_tokens, 0);
        assert_eq!(usage.output_tokens, 4);
        let split = usage_from_chat(&ChatUsage {
            prompt_tokens: 30,
            prompt_cache_hit_tokens: 20,
            prompt_cache_miss_tokens: 10,
            completion_tokens: 1,
            ..ChatUsage::default()
        });
        assert_eq!(split.input_tokens, 10);
        assert_eq!(split.cache_read_tokens, 20);
        assert_eq!(split.cache_creation_tokens, 0);
    }

    #[test]
    fn openai_cached_tokens_count_as_a_read() {
        let raw: ChatUsage = serde_json::from_str(
            r#"{"prompt_tokens":30,"completion_tokens":4,"prompt_tokens_details":{"cached_tokens":20}}"#,
        )
        .unwrap();
        let usage = usage_from_chat(&raw);
        assert_eq!(usage.cache_read_tokens, 20);
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.cache_creation_tokens, 0);
    }

    #[test]
    fn deepseek_hit_miss_wins_when_both_shapes_are_present() {
        let usage = usage_from_chat(&ChatUsage {
            prompt_tokens: 30,
            prompt_cache_hit_tokens: 20,
            prompt_cache_miss_tokens: 10,
            prompt_tokens_details: crate::chat::PromptTokensDetails { cached_tokens: 20 },
            completion_tokens: 1,
        });
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.cache_read_tokens, 20);
        assert_eq!(usage.cache_creation_tokens, 0);
    }

    #[test]
    fn tool_json_is_stable_when_the_caller_shuffles_names() {
        let read = ToolDef {
            name: "read_file".to_string(),
            description: "read".to_string(),
            input_schema: Default::default(),
            required: vec!["path".to_string()],
        };
        let bash = ToolDef {
            name: "bash".to_string(),
            description: "bash".to_string(),
            input_schema: Default::default(),
            required: Vec::new(),
        };
        let forward = to_tools(&[read.clone(), bash.clone()]);
        let backward = to_tools(&[bash, read]);
        assert_eq!(forward, backward);
        assert_eq!(forward[0]["function"]["name"], "bash");
    }

    #[test]
    fn system_message_stays_the_first_prefix_byte() {
        let llm = provider("deepseek-flash");
        let once = llm.to_messages(&[]);
        let twice = llm.to_messages(&[]);
        assert_eq!(once[0].role, "system");
        assert_eq!(once[0].content, twice[0].content);
        let text = once[0].content.as_ref().unwrap().as_str().unwrap();
        assert!(!text.contains("timestamp"));
    }
}
