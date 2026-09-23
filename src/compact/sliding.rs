use crate::api::Message;
use crate::compact::{safe_split_point, CompactionStrategy};
use crate::provider::Provider;

/// 第 07 课：只留最后 `keep_last` 条。不打模型。切分走 `safe_split_point`。
pub struct SlidingWindow {
    pub keep_last: usize,
}

impl CompactionStrategy for SlidingWindow {
    fn compact(&self, messages: &[Message], _llm: &dyn Provider) -> Result<Vec<Message>, String> {
        if messages.len() <= self.keep_last {
            return Ok(messages.to_vec());
        }
        let split = safe_split_point(messages, messages.len() - self.keep_last);
        Ok(messages[split..].to_vec())
    }
}
