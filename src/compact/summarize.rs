use crate::api::{render_transcript, BlockType, Message};
use crate::compact::{safe_split_point, CompactionStrategy};
use crate::provider::Provider;

const DEFAULT_INSTRUCTIONS: &str = "Summarize the following conversation concisely. \
Preserve facts, decisions, file paths, code identifiers, and anything else \
needed to continue the conversation. Output the summary directly with no preamble.";

/// 第 07 课：越过 `threshold` 后，让**同一个** Provider 把旧半段收成一条合成 user。
/// 这一次 `send` 不带 tools，单轮文本。压缩在循环外，避免套进 `send` 里递归。
pub struct Summarize {
    pub threshold: usize,
    pub keep_recent: usize,
    pub instructions: String,
}

impl CompactionStrategy for Summarize {
    fn compact(&self, messages: &[Message], llm: &dyn Provider) -> Result<Vec<Message>, String> {
        if messages.len() < self.threshold {
            return Ok(messages.to_vec());
        }
        let desired = messages.len().saturating_sub(self.keep_recent);
        let split = safe_split_point(messages, desired);
        if split == 0 {
            return Ok(messages.to_vec());
        }
        let old = &messages[..split];
        let recent = &messages[split..];

        let instructions = if self.instructions.is_empty() {
            DEFAULT_INSTRUCTIONS
        } else {
            self.instructions.as_str()
        };
        let prompt = format!("{instructions}\n\n{}", render_transcript(old));
        let resp = llm
            .send(&[Message::user_text(prompt)], &[])
            .map_err(|err| format!("summarize: {err}"))?;

        let summary = resp
            .content
            .iter()
            .find(|block| block.ty == BlockType::Text && !block.text.is_empty())
            .map(|block| block.text.clone())
            .ok_or_else(|| "summarize: empty response".to_string())?;

        println!("[compacted {} messages → summary]", old.len());
        let mut out = vec![Message::user_text(format!(
            "[earlier conversation summary]\n{summary}"
        ))];
        out.extend_from_slice(recent);
        Ok(out)
    }
}
