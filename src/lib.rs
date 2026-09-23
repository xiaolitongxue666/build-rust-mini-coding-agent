//! 已完成课的共享实现。单课 demo 在 `examples/`，总体 bin 在 `src/main.rs`。
//! 不要在 example 里再抄一份循环。第 03 课起循环只认 `Provider`。
//!
//! 第 10 课：课上把 Go 的扁平 `package main` 拆进 `internal/`。
//! Rust 已经是 [Cargo 包布局](https://doc.rust-lang.org/cargo/guide/project-layout.html)：
//! `src/lib.rs` 库 + `src/main.rs` bin + `examples/`。模块默认私有，
//! `publish = false` 就是「不是给外人 import 的库」——不要再套 `src/internal/`。
//! 模块树按[领域](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)分，不按文件类型分。
//!
//! ```text
//! src/main.rs          接线（对齐课上 main.go）
//! src/lib.rs           crate 根
//! src/api.rs           通用类型，不依赖本 crate 其它模块
//! src/provider/        Provider + DeepSeek / Mock
//! src/chat.rs          线协议，仅适配器；crate 内可见
//! src/tools/           Tool + Registry + 各工具一文件
//! src/compact/         CompactionStrategy
//! src/ui/              banner / spinner / 一次性输入
//! src/agent.rs         内层循环
//! src/repl.rs          外层 REPL
//! src/commands.rs      斜杠（碰到所有扩展点，留在集成层）
//! src/gate.rs          权限门
//! src/conversation.rs  第 06 课切片辅助
//! ```
//!
//! 依赖方向：`api` 在底。逻辑认 api。UI 可以认逻辑，不要反向。现在没有环。
//! 第 11 课子 agent 才可能出现 agent ↔ ui ↔ subagent。本课不拆。

pub mod agent;
pub mod api;
pub(crate) mod chat;
pub mod commands;
pub mod compact;
pub mod conversation;
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
    use crate::conversation::{messages_json, tool_pairs_intact};
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

    #[test]
    fn chat_input_down_restores_draft() {
        let mut state = ui::ChatInputState::new(40, vec!["a".into()]);
        state.apply(ui::ChatKey::Char('z'));
        state.apply(ui::ChatKey::Up);
        state.apply(ui::ChatKey::Down);
        assert_eq!(state.value(), "z");
    }

    #[test]
    fn slash_exit_is_quit() {
        let mut llm = MockProvider::text("x");
        let tools = tools::default_tool_defs();
        let mut messages = Vec::new();
        let compact = compact::NoCompaction;
        let mut verbose = false;
        let mut ctx = commands::CommandCtx {
            llm: &mut llm,
            messages: &mut messages,
            tools: &tools,
            compact: &compact,
            verbose: &mut verbose,
        };
        assert_eq!(
            commands::run_command("/exit", &mut ctx),
            Some(commands::CommandOutcome::Quit)
        );
    }

    #[test]
    fn slash_clear_empties_messages() {
        let mut llm = MockProvider::text("x");
        let tools = tools::default_tool_defs();
        let mut messages = vec![api::Message::user_text("hi")];
        let compact = compact::NoCompaction;
        let mut verbose = false;
        let mut ctx = commands::CommandCtx {
            llm: &mut llm,
            messages: &mut messages,
            tools: &tools,
            compact: &compact,
            verbose: &mut verbose,
        };
        assert_eq!(
            commands::run_command("/clear", &mut ctx),
            Some(commands::CommandOutcome::Handled)
        );
        assert!(messages.is_empty());
    }

    // 第 06 课：system 在 Provider / 线协议上，不在通用切片里。
    #[test]
    fn conversation_system_stays_off_slice() {
        let messages = vec![Message::user_text("hi")];
        let wire = to_messages("you are deepseek", &messages);
        assert_eq!(messages.len(), 1);
        assert_eq!(wire[0].role, "system");
        assert_eq!(wire[1].role, "user");
        assert_eq!(wire.len(), 2);

        let empty = to_messages("you are deepseek", &[]);
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].role, "system");
    }

    #[test]
    fn conversation_dump_and_pairing() {
        let messages = vec![
            Message::user_text("hi"),
            Message::assistant(vec![Block::tool_use("toolu_01abc", "bash", "{}")]),
            Message::tool_results(vec![Block::tool_result("toolu_01abc", "ok", false)]),
        ];
        assert!(tool_pairs_intact(&messages));
        let dump = messages_json(&messages);
        assert!(dump.contains("tool_use"), "{dump}");
        assert!(dump.contains("toolu_01abc"), "{dump}");
    }

    #[test]
    fn render_transcript_includes_block_kinds() {
        let messages = vec![
            Message::user_text("hello"),
            Message::assistant(vec![
                Block::text("thinking…"),
                Block::tool_use("t", "bash", r#"{"cmd":"ls"}"#),
            ]),
            Message::tool_results(vec![Block::tool_result("t", "file.txt", false)]),
        ];
        let out = api::render_transcript(&messages);
        for want in [
            "user:",
            "assistant:",
            "hello",
            "called bash",
            r#"{"cmd":"ls"}"#,
            "tool result",
            "file.txt",
        ] {
            assert!(out.contains(want), "missing {want:?}\n{out}");
        }
    }
}
