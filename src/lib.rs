//! 已完成课的共享实现。单课 demo 在 `examples/`，总体 bin 在 `main.rs`。
//! 不要在 example 里再抄一份循环。第 03 课起循环只认 `Provider`。

pub mod agent;
pub mod api;
pub mod chat;
pub mod gate;
pub mod provider;
pub mod repl;
pub mod tools;
pub mod ui;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::api::{Block, Message, Role, StopReason};
    use crate::provider::deepseek::{from_stop_reason, to_messages, to_tools};
    use crate::provider::{MockProvider, Provider};

    fn tmp_path(suffix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("byo-lib-{nanos}-{suffix}"))
    }

    fn lines_of(s: &str) -> std::io::Lines<std::io::Cursor<Vec<u8>>> {
        std::io::Cursor::new(s.as_bytes().to_vec()).lines()
    }

    #[test]
    fn chat_url_uses_official_completions_path() {
        assert_eq!(
            chat::chat_completions_url("https://api.deepseek.com"),
            "https://api.deepseek.com/chat/completions"
        );
    }

    #[test]
    fn redact_error_keeps_type_not_key() {
        let raw = r#"{"error":{"message":"Authentication Fails, Your api key: SECRET is invalid","type":"authentication_error","code":"invalid_request_error"}}"#;
        let out = chat::redact_api_error(raw);
        assert!(out.contains("authentication_error"), "{out}");
        assert!(!out.contains("SECRET"), "{out}");
    }

    #[test]
    fn confirm_default_is_no() {
        for raw in ["\n", "n\n", "no\n", "maybe\n", ""] {
            let mut lines = lines_of(raw);
            assert!(!gate::confirm("approve?", &mut lines), "input={raw:?}");
        }
    }

    #[test]
    fn confirm_yes_variants() {
        for raw in ["y\n", "Y\n", " yes \n", "YES\n"] {
            let mut lines = lines_of(raw);
            assert!(gate::confirm("approve?", &mut lines), "input={raw:?}");
        }
    }

    #[test]
    fn dispatch_write_then_read_roundtrip() {
        let path = tmp_path("roundtrip.txt");
        let write_in = serde_json::json!({
            "path": path,
            "content": "haiku"
        })
        .to_string();
        let (wrote, write_err) = tools::dispatch_tool("write_file", &write_in);
        assert!(!write_err, "{wrote}");

        let read_in = serde_json::json!({ "path": path }).to_string();
        let (body, read_err) = tools::dispatch_tool("read_file", &read_in);
        let _ = std::fs::remove_file(&path);
        assert!(!read_err, "{body}");
        assert_eq!(body, "haiku");
    }

    #[test]
    fn json_field_missing_is_error_string() {
        let err = tools::json_field(r#"{"path":"x"}"#, "command").unwrap_err();
        assert!(err.contains("command"), "{err}");
    }

    // 第 03 课：空历史不能 panic。Mock 不打网。
    #[test]
    fn agent_loop_empty_messages_with_mock() {
        let mut llm = MockProvider::text("ok");
        let mut lines = lines_of("");
        let out = agent::agent_loop(&mut llm, &[], Vec::new(), &mut lines, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, Role::Assistant);
        assert_eq!(out[0].content[0].text, "ok");
    }

    #[test]
    fn finish_reason_maps_to_generic_stop() {
        assert_eq!(from_stop_reason("tool_calls"), StopReason::ToolUse);
        assert_eq!(from_stop_reason("stop"), StopReason::EndTurn);
        assert_eq!(from_stop_reason("length"), StopReason::Other);
    }

    #[test]
    fn deepseek_to_messages_splits_tool_result() {
        let messages = vec![
            Message::user_text("hi"),
            Message::assistant(vec![Block::tool_use(
                "call-1",
                "bash",
                r#"{"command":"pwd"}"#,
            )]),
            Message::tool_results(vec![Block::tool_result("call-1", "/tmp", false)]),
        ];
        let wire = to_messages("you are deepseek", &messages);
        assert_eq!(wire[0].role, "system");
        assert_eq!(wire[1].role, "user");
        assert_eq!(wire[2].role, "assistant");
        assert!(wire[2].tool_calls.as_ref().unwrap()[0].id == "call-1");
        assert_eq!(wire[3].role, "tool");
        assert_eq!(wire[3].tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn deepseek_to_tools_wraps_function_schema() {
        let tools = tools::default_tool_defs();
        let value = to_tools(&tools);
        assert_eq!(value[0]["type"], "function");
        assert_eq!(value[0]["function"]["name"], "bash");
        assert_eq!(value.as_array().map(|a| a.len()), Some(3));
    }

    #[test]
    fn mock_set_model() {
        let mut llm = MockProvider::text("x");
        llm.set_model("other".to_string());
        assert_eq!(llm.model(), "other");
    }

    #[test]
    fn banner_wide_uses_block_art() {
        assert_eq!(ui::BIG_BANNER_WIDTH, 66);
        let text = ui::banner_text(80);
        assert!(text.contains('█'), "{text}");
        assert!(text.contains("DeepSeek"), "{text}");
    }

    #[test]
    fn banner_narrow_uses_wordmark() {
        let text = ui::banner_text(40);
        assert!(text.contains("RUSTBYO"), "{text}");
        assert!(!text.contains('█'), "{text}");
    }

    #[test]
    fn ctrl_c_once_clears_twice_quits() {
        assert_eq!(ui::ctrl_c_action(1), ui::CtrlCAction::Clear);
        assert_eq!(ui::ctrl_c_action(2), ui::CtrlCAction::Quit);
        assert_eq!(ui::ctrl_c_action(3), ui::CtrlCAction::Quit);
    }
}
