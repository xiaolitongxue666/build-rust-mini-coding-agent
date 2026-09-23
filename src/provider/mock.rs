//! 第 03 课：固定回复的 Provider。测循环、门、分发，不打真实模型、不读密钥。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::api::{Block, Message, Response, StopReason, ToolDef, Usage};
use crate::provider::Provider;

#[derive(Clone)]
pub struct MockProvider {
    model: String,
    system: String,
    responses: Vec<Response>,
    next: Arc<AtomicUsize>,
    /// 第 06 课：记下每次 `send` 收到的整段切片。模型没有会话，客户端每次重发。
    sent: Arc<Mutex<Vec<Vec<Message>>>>,
}

impl MockProvider {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            model: "mock".to_string(),
            system: String::new(),
            responses: vec![Response {
                content: vec![Block::text(body)],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            }],
            next: Arc::new(AtomicUsize::new(0)),
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_responses(responses: Vec<Response>) -> Self {
        Self {
            model: "mock".to_string(),
            system: String::new(),
            responses,
            next: Arc::new(AtomicUsize::new(0)),
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn sent(&self) -> Vec<Vec<Message>> {
        self.sent.lock().expect("mock sent lock").clone()
    }
}

impl Provider for MockProvider {
    fn send(&self, messages: &[Message], _tools: &[ToolDef]) -> Result<Response, String> {
        self.sent
            .lock()
            .expect("mock sent lock")
            .push(messages.to_vec());
        if self.responses.is_empty() {
            return Err("mock has no responses".to_string());
        }
        let i = self.next.fetch_add(1, Ordering::SeqCst);
        let idx = i.min(self.responses.len() - 1);
        Ok(self.responses[idx].clone())
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, name: String) {
        self.model = name;
    }

    fn system(&self) -> &str {
        &self.system
    }

    fn set_system(&mut self, prompt: String) {
        self.system = prompt;
    }
}
