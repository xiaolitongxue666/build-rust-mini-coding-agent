//! 第 03 课：固定回复的 Provider。测循环、门、分发，不打真实模型、不读密钥。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::api::{Block, Message, Response, StopReason, ToolDef};
use crate::provider::Provider;

#[derive(Clone)]
pub struct MockProvider {
    model: String,
    responses: Vec<Response>,
    next: Arc<AtomicUsize>,
}

impl MockProvider {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            model: "mock".to_string(),
            responses: vec![Response {
                content: vec![Block::text(body)],
                stop_reason: StopReason::EndTurn,
            }],
            next: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_responses(responses: Vec<Response>) -> Self {
        Self {
            model: "mock".to_string(),
            responses,
            next: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Provider for MockProvider {
    fn send(&self, _messages: &[Message], _tools: &[ToolDef]) -> Result<Response, String> {
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
}
