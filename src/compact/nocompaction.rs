use crate::api::Message;
use crate::compact::CompactionStrategy;
use crate::provider::Provider;

/// 第 07 课默认：不改切片。预算在别处控。
pub struct NoCompaction;

impl CompactionStrategy for NoCompaction {
    fn compact(&self, messages: &[Message], _llm: &dyn Provider) -> Result<Vec<Message>, String> {
        Ok(messages.to_vec())
    }
}
