//! 第 15 课：[Project context with AGENTS.md](https://www.byoharness.dev/chapters/15-agents-md.html)
//!
//! 本课小 demo：把 cwd 的 `AGENTS.md` 拼进 system。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! system_prompt（行为） + AGENTS.md（这个仓库）  →  每次请求的 system
//! ```
//!
//! 启动时读一次。缺文件不打日志。不往上找目录，不热重载。
//! `/model` 和 `/clear` 都不拆掉这段。`/clear` 只清 `messages`。
//!
//! 课上的 `/context` 和 `/reload-context` 是练习，这里不写。
//! 不要把密钥写进 `AGENTS.md`。
//!
//! 启动：`bash scripts/run.sh 15`。总体：`bash scripts/run.sh`。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use build_rust_mini_coding_agent::agents_context::{load_agents_context_from, CONTEXT_HEADER};
    use build_rust_mini_coding_agent::provider::{DeepSeekProvider, Provider};

    #[test]
    fn lesson_15_missing_file_adds_nothing() {
        let path = std::env::temp_dir().join("byo-lesson15-example-missing.md");
        let _ = fs::remove_file(&path);
        assert_eq!(load_agents_context_from(&path), "");
    }

    #[test]
    fn lesson_15_file_is_context_not_a_second_copy() {
        let path = std::env::temp_dir().join("byo-lesson15-example.md");
        fs::write(&path, "use tabs\n").unwrap();
        let context = load_agents_context_from(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(context, format!("{CONTEXT_HEADER}use tabs\n"));

        let mut llm = DeepSeekProvider::new("k", "deepseek-flash", "https://api.deepseek.com");
        let behavior = llm.system().to_string();
        llm.attach_project_context(context);
        llm.set_system(behavior);
        assert_eq!(llm.system().matches("# Project context").count(), 1);
        assert!(llm.system().contains("use tabs"));
        llm.set_model("deepseek-chat".into());
        assert!(llm.system().contains("use tabs"));
        assert!(llm.system().contains("deepseek-chat"));
    }
}
