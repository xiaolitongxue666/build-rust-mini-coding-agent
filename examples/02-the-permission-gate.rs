//! 第 02 课：[The permission gate](https://www.byoharness.dev/chapters/02-the-permission-gate.html)
//!
//! 本课小 demo：第 01 课循环 + harness 层 `approve?`。实现在 crate 库里一份，
//! 这里只接线 `run_repl(true)`。总体 `src/main.rs` 也走这道门。
//!
//! ```text
//! [调用模型]
//!     │
//!     ▼
//! [有 tool_calls?]─no──▶ [打印文本，回到 REPL]
//!     │
//!    yes
//!     │
//!     ▼
//! [打印 [tool] name args]
//!     │
//!     ▼
//! [approve? y/n] ──n──▶ [append "user denied…" is_error] ──▶ 再调模型
//!     │
//!    y
//!     │
//!     ▼
//! [真正 dispatch]
//! ```
//!
//! 审批放 harness，不放工具里、也不放系统提示里。Go 用全局 Scanner；
//! Rust 把 REPL 的 `lines` 传进 `confirm`。再 lock 一次会抢字节。
//!
//! 拒绝必须是 `("user denied this tool call", true)`。模型在循环里，
//! 失败是下一轮输入，不是异常。
//!
//! 启动：`bash scripts/run.sh 02`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use build_rust_mini_coding_agent::gate::execute_gated;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(suffix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("byo-lesson02-{nanos}-{suffix}"))
    }

    fn lines_of(s: &str) -> std::io::Lines<std::io::Cursor<Vec<u8>>> {
        std::io::Cursor::new(s.as_bytes().to_vec()).lines()
    }

    // 第 02 课：拒绝必须是 is_error，且不能真的跑工具。
    #[test]
    fn lesson_02_deny_is_error_result_and_does_not_run() {
        let path = tmp_path("should-not-exist.txt");
        let input = serde_json::json!({
            "path": path,
            "content": "nope"
        })
        .to_string();
        let mut lines = lines_of("n\n");
        let (text, is_err) = execute_gated("write_file", &input, &mut lines);
        assert!(is_err);
        assert_eq!(text, "user denied this tool call");
        assert!(!path.exists(), "denied write must not create the file");
    }

    #[test]
    fn lesson_02_approve_then_unknown_tool_is_error_result() {
        let mut lines = lines_of("y\n");
        let (text, is_err) = execute_gated("not_a_tool", "{}", &mut lines);
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }
}
