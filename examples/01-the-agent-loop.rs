//! 第 01 课：[The agent loop](https://www.byoharness.dev/chapters/01-the-agent-loop.html)
//!
//! 本课小 demo：外层 REPL + 内层 tool-use，**没有**权限门。实现在 crate 库里一份，
//! 这里只接线 `run_repl(false)`。总体入口是 `src/main.rs`（带第 02 课的门）。
//!
//! ```text
//! [你的输入]
//!     │
//!     ▼
//! [append 到 messages]
//!     │
//!     ▼
//! [调用模型] ─────────┐
//!     │               │
//!     ▼               │
//! [有 tool_calls?]─no─┴──▶ [打印文本，回到 REPL]
//!     │
//!    yes
//!     │
//!     ▼
//! [执行每个工具]   ← 本课立刻 dispatch，不问 approve
//!     │
//!     ▼
//! [append tool 结果]
//!     │
//!     ▼
//! (回到「调用模型」)
//! ```
//!
//! | 循环 | 谁在推 | 一轮是什么 |
//! |---|---|---|
//! | 外层 REPL | 你的键盘 | 读一行 → `agent_loop` → 等下一行 |
//! | 内层 agent | 模型的选择 | 调模型 → 若有 tool_calls 则执行并 append → 重复直到停 |
//!
//! 课程正文用 Anthropic Messages 词表。本仓库 live 走 DeepSeek Chat Completions。
//!
//! | 课程（Anthropic） | 本文件（DeepSeek / OpenAI） | 含义 |
//! |---|---|---|
//! | `messages` | `messages` | 到目前为止的全部对话；API 无状态，客户端带着走 |
//! | `tools` + `input_schema` | `tools` + `function.parameters` | 模型能调用的操作面（JSON Schema） |
//! | content 里的 `tool_use` | `message.tool_calls` | 模型要 harness 在本地跑某个工具 |
//! | `stop_reason: tool_use` | `finish_reason: tool_calls` | 内层继续转 |
//! | `stop_reason: end_turn` | `finish_reason: stop` | 打出文本，回到 REPL |
//! | `tool_result` + `tool_use_id` | `role: "tool"` + `tool_call_id` | 必须对上 id，对不上 API 400 |
//!
//! 启动：`bash scripts/run.sh 01`。不要在这个文件里写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(false);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::gate::execute_direct;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(suffix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("byo-lesson01-{nanos}-{suffix}"))
    }

    // 第 01 课：没有门，write 会真的落盘。
    #[test]
    fn lesson_01_direct_write_then_read() {
        let path = tmp_path("roundtrip.txt");
        let write_in = serde_json::json!({
            "path": path,
            "content": "haiku"
        })
        .to_string();
        let (wrote, write_err) = execute_direct("write_file", &write_in);
        assert!(!write_err, "{wrote}");

        let read_in = serde_json::json!({ "path": path }).to_string();
        let (body, read_err) = execute_direct("read_file", &read_in);
        let _ = std::fs::remove_file(&path);
        assert!(!read_err, "{body}");
        assert_eq!(body, "haiku");
    }

    #[test]
    fn lesson_01_unknown_tool_is_error_result() {
        let (text, is_err) = execute_direct("not_a_tool", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }

    #[test]
    fn lesson_01_read_missing_file_is_error_result() {
        let (text, is_err) =
            execute_direct("read_file", r#"{"path":"/does/not/exist-byo-lesson01"}"#);
        assert!(is_err);
        assert!(!text.is_empty());
    }
}
