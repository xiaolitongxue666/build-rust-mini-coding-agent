//! 第 15 课：[Project context with AGENTS.md](https://www.byoharness.dev/chapters/15-agents-md.html)
//!
//! 行为约束在 `system_prompt`。这一份是**当前项目**的上下文，拼进 system，不当成用户消息。
//! 启动时读一次 cwd 下的 `AGENTS.md`，和 `mcp.json` 一样相对工作目录，不往上找，不热重载。
//! 缺文件是正常情况：返回空串，不打日志。
//!
//! 课上的 `/context`、`/reload-context` 是练习，这里不写。

use std::path::Path;

/// 课上那句软标记。没有它，模型会把项目上下文当成和 harness 行为同一级的指令，有时会复述「仓库里写着」。
pub const CONTEXT_HEADER: &str = "\n\n# Project context (from AGENTS.md)\n\n";

pub fn load_agents_context() -> String {
    load_agents_context_from(Path::new("AGENTS.md"))
}

pub fn load_agents_context_from(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(body) => format!("{CONTEXT_HEADER}{body}"),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_empty() {
        let path = std::env::temp_dir().join("byo-lesson15-missing-AGENTS.md");
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_agents_context_from(&path), "");
    }

    #[test]
    fn present_file_is_appended_under_the_header() {
        let path = std::env::temp_dir().join("byo-lesson15-AGENTS.md");
        std::fs::write(&path, "use tabs\n").unwrap();
        let text = load_agents_context_from(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(text, format!("{CONTEXT_HEADER}use tabs\n"));
        assert_eq!(text.matches("# Project context").count(), 1);
    }
}
