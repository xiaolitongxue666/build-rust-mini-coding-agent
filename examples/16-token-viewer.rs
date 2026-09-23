//! 第 16 课：[The token viewer](https://www.byoharness.dev/chapters/16-token-viewer.html)
//!
//! 本课小 demo：累计 token，按 DeepSeek 分时段价格显示人民币。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! 每次响应的 usage ──按那一刻的北京时间入账──▶ /tokens
//!                                    └──────▶ 空闲状态栏
//! ```
//!
//! 高峰（北京时间，左闭右开）：周一到周五 09:00–12:00、14:00–18:00。
//! 周末、2026 法定节假日全天空闲。调休上班日按工作日。
//! 钟点固定 UTC+8，不跟本机时区走。
//!
//! 金额前缀 `~`。未知模型显示「未知模型」，不显示 ¥0。
//! cache 行为 0 时不打印。Mock 不报用量。
//!
//! 启动：`bash scripts/run.sh 16`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::api::{Message, Usage};
    use build_rust_mini_coding_agent::commands::{run_command, CommandCtx, CommandOutcome};
    use build_rust_mini_coding_agent::pricing::{self, Beijing};
    use build_rust_mini_coding_agent::provider::{DeepSeekProvider, MockProvider, Provider};

    fn off_peak() -> Beijing {
        Beijing {
            year: 2026,
            month: 9,
            day: 21,
            hour: 20,
        }
    }

    #[test]
    fn lesson_16_flash_off_peak_miss_is_one_yuan() {
        let llm = DeepSeekProvider::new("k", "deepseek-flash", "https://api.deepseek.com");
        llm.record_usage_at(
            Usage {
                input_tokens: 1_000_000,
                ..Usage::default()
            },
            off_peak(),
        );
        let report = llm.token_report().expect("deepseek reports usage");
        assert!((report.yuan - 1.0).abs() < 1e-9);
        let text = pricing::format_tokens(Some(&report));
        assert!(text.contains("~¥1.0000"), "{text}");
        assert!(!text.contains("cache hit"), "{text}");
        assert!(text.contains("空闲"), "{text}");
        assert!(!text.contains("高峰"), "{text}");
    }

    #[test]
    fn lesson_16_unknown_model_is_not_zero_yuan() {
        let llm = DeepSeekProvider::new("k", "not-a-model", "https://api.deepseek.com");
        llm.record_usage_at(
            Usage {
                input_tokens: 100,
                ..Usage::default()
            },
            off_peak(),
        );
        let text = pricing::format_tokens(llm.token_report().as_ref());
        assert!(text.contains("未知模型"), "{text}");
        assert!(!text.contains('¥'), "{text}");
    }

    #[test]
    fn lesson_16_mock_does_not_invent_a_bill() {
        let mut llm = MockProvider::text("x");
        assert!(llm.token_report().is_none());
        let mut messages = Vec::<Message>::new();
        let tools = build_rust_mini_coding_agent::tools::default_tool_defs();
        let compact = build_rust_mini_coding_agent::compact::NoCompaction;
        let mut verbose = false;
        let mut ctx = CommandCtx {
            llm: &mut llm,
            messages: &mut messages,
            tools: &tools,
            compact: &compact,
            verbose: &mut verbose,
            subagents: build_rust_mini_coding_agent::commands::no_subagents(),
        };
        assert_eq!(
            run_command("/tokens", &mut ctx),
            Some(CommandOutcome::Handled)
        );
        assert!(pricing::format_tokens(None).contains("doesn't report"));
    }
}
