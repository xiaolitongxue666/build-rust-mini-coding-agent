//! 第 03 课：LLM 后端接口。循环只认 `Provider`，换实现就换供应商。
//!
//! ```text
//! var llm Provider = DeepSeekProvider::new(...)
//! ```
//!
//! 课上的参考实现是 Anthropic。本仓库 live 是 DeepSeek（OpenAI 兼容）。
//! 本地 Ollama / LM Studio 也走同一套 Chat Completions，改 `base_url` 即可，
//! 不必第三份适配器。Anthropic SDK 不进默认路径。
//!
//! `Model` / `SetModel` 是给第 05 课 `/model` 的小让步；本课不写斜杠命令。

pub(crate) mod deepseek;
mod mock;

pub use deepseek::DeepSeekProvider;
pub use mock::MockProvider;

use crate::api::{Message, Response, ToolDef};

pub trait Provider: Send {
    fn send(&self, messages: &[Message], tools: &[ToolDef]) -> Result<Response, String>;
    fn model(&self) -> &str;
    fn set_model(&mut self, name: String);
}
