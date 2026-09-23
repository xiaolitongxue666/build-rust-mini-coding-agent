//! 第 17 课：[Prompt caching](https://www.byoharness.dev/chapters/17-prompt-caching.html)
//!
//! 本课小 demo：让重复发送的前缀能被 DeepSeek 磁盘缓存命中。实现在 crate 库一份，这里只接线。
//!
//! 课上给 Claude 的 system 加一行 `cache_control`。DeepSeek 没有这个字段，缓存默认开着：
//! <https://api-docs.deepseek.com/guides/kv_cache>
//! 命中条件是前缀单元完整相同。能做的是把不变的字节稳住：
//!
//! - system 固定在第一条消息，里面不放时间或随机数
//! - 工具按名字排序后再编码
//! - 压缩只改 `messages`，不改 system
//!
//! `/tokens` 已经会打印 cache hit。DeepSeek 不另收写入加价，所以没有 cache write 那一行。
//!
//! 启动：`bash scripts/run.sh 17`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::api::{Message, Usage};
    use build_rust_mini_coding_agent::compact::{CompactionStrategy, SlidingWindow};
    use build_rust_mini_coding_agent::pricing::{self, Beijing};
    use build_rust_mini_coding_agent::provider::{DeepSeekProvider, Provider};
    use build_rust_mini_coding_agent::tools::default_registry;

    #[test]
    fn lesson_17_tool_names_stay_sorted() {
        let names: Vec<String> = default_registry()
            .definitions()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn lesson_17_compaction_leaves_the_system_prefix() {
        let llm = DeepSeekProvider::new("k", "deepseek-flash", "https://api.deepseek.com");
        let before = llm.system().to_string();
        let messages: Vec<Message> = (0..6)
            .map(|index| Message::user_text(format!("turn {index}")))
            .collect();
        let after = SlidingWindow { keep_last: 2 }
            .compact(&messages, &llm)
            .unwrap();
        assert!(after.len() < messages.len());
        assert_eq!(llm.system(), before);
    }

    #[test]
    fn lesson_17_cache_hit_shows_up_without_a_write_premium() {
        let llm = DeepSeekProvider::new("k", "deepseek-flash", "https://api.deepseek.com");
        llm.record_usage_at(
            Usage {
                cache_read_tokens: 1_000_000,
                ..Usage::default()
            },
            Beijing {
                year: 2026,
                month: 9,
                day: 21,
                hour: 20,
            },
        );
        let text = pricing::format_tokens(llm.token_report().as_ref());
        assert!(text.contains("cache hit"), "{text}");
        assert!(!text.contains("cache write"), "{text}");
        assert!(text.contains("~¥0.0200"), "{text}");
    }
}
