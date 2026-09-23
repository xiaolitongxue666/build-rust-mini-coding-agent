//! 第 18 课：[Diff approval for writes](https://www.byoharness.dev/chapters/18-diff-approval.html)
//!
//! 本课小 demo：`write_file` 落盘前先给统一 diff。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! tool_use write_file  →  读磁盘上的旧文件  →  统一 diff  →  一次 y/n  →  才写入
//! ```
//!
//! 模型看不到这份 diff。拒绝时回的仍是 `user denied this tool call`。
//! 只认本地名字 `write_file`。MCP 写文件工具继续是普通 `approve?`。
//!
//! 整屏里 diff 占视口，`+` 绿、`-` 红。课上用 Chroma；这里继续 crossterm。
//! 管道模式把同一份 diff 打在 y/n 前面。
//!
//! 启动：`bash scripts/run.sh 18`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use build_rust_mini_coding_agent::ui::{Harness, HarnessEvent};
    use build_rust_mini_coding_agent::write_diff::{build_write_diff, write_approval};

    #[test]
    fn lesson_18_new_file_is_all_additions() {
        let path = std::env::temp_dir().join("byo-lesson18-example-new.txt");
        let _ = fs::remove_file(&path);
        let raw = format!(
            r#"{{"path":"{}","content":"hello\n"}}"#,
            path.display().to_string().replace('\\', "\\\\")
        );
        let (prompt, detail) = write_approval("write_file", &raw);
        assert!(prompt.contains("approve write to"), "{prompt}");
        assert!(detail.contains("--- /dev/null"), "{detail}");
        assert!(detail.contains("+hello"), "{detail}");
    }

    #[test]
    fn lesson_18_same_bytes_say_no_changes() {
        let path = std::env::temp_dir().join("byo-lesson18-example-same.txt");
        fs::write(&path, "same\n").unwrap();
        let text = build_write_diff(path.to_str().unwrap(), "same\n");
        let _ = fs::remove_file(&path);
        assert_eq!(text, "(no changes)\n");
    }

    #[test]
    fn lesson_18_mcp_write_stays_a_plain_prompt() {
        let (prompt, detail) =
            write_approval("filesystem_write_file", r#"{"path":"a.txt","content":"x"}"#);
        assert_eq!(prompt, "approve?");
        assert!(detail.is_empty());
    }

    #[test]
    fn lesson_18_modal_shows_the_diff_and_one_choice() {
        let mut harness = Harness::new(40, 16, "banner\n".into(), Vec::new());
        let _ = harness.apply(HarnessEvent::Approval {
            prompt: "approve write to haiku.txt?".into(),
            detail: "--- /dev/null\n+++ haiku.txt (new file)\n@@ -0,0 +1,1 @@\n+hello\n".into(),
        });
        let view = harness.view(&[]);
        assert!(view.contains("+hello"), "{view}");
        assert!(view.contains("approve write to haiku.txt?"), "{view}");
        assert!(view.contains('╭'), "{view}");
    }
}
