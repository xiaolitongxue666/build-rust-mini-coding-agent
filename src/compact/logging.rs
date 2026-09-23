use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::{render_transcript, Message};
use crate::compact::CompactionStrategy;
use crate::provider::Provider;

/// 第 07 课装饰器：包任意策略，长度变了才写 before/after。
/// 日志和策略正交，不要在每个策略上加 `LogTo`。
pub struct LoggingStrategy {
    pub inner: Box<dyn CompactionStrategy>,
    pub file_path: String,
}

pub fn with_logging(
    inner: impl CompactionStrategy + 'static,
    path: impl Into<String>,
) -> LoggingStrategy {
    LoggingStrategy {
        inner: Box::new(inner),
        file_path: path.into(),
    }
}

impl CompactionStrategy for LoggingStrategy {
    fn compact(&self, messages: &[Message], llm: &dyn Provider) -> Result<Vec<Message>, String> {
        let after = self.inner.compact(messages, llm)?;
        if after.len() != messages.len() && !self.file_path.is_empty() {
            if let Err(err) = write_event(&self.file_path, messages, &after) {
                println!("compaction log write failed: {err}");
            }
        }
        Ok(after)
    }
}

fn write_event(path: &str, before: &[Message], after: &[Message]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    writeln!(file, "=========================")?;
    writeln!(file, "[{stamp}] compaction event")?;
    writeln!(
        file,
        "BEFORE ({} messages):\n{}",
        before.len(),
        render_transcript(before)
    )?;
    writeln!(file, "---")?;
    writeln!(
        file,
        "AFTER ({} messages):\n{}",
        after.len(),
        render_transcript(after)
    )?;
    Ok(())
}
