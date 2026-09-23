//! 第 11 课：[Subagents](https://www.byoharness.dev/chapters/11-subagents.html)
//!
//! 本课两步重构 + 一个能力：
//!
//! 1. `agent_loop` 收成 `Agent` 结构体。根和子 agent 各一份状态。
//! 2. `Subagent` 接口 + Registry。和 Provider / Tool / 压缩同一形状。
//! 3. `DelegateTool` 把子 agent 暴露成 `delegate_research`。
//!
//! 子 agent 是**另一次**循环：自己的切片、自己的工具子集、自己的 system。
//! 读 20 个文件只回一条 tool_result，主对话不烂。
//!
//! `DelegateTool` 不进 `src/tools/`。课上的环：`tool → subagent → agent → tool`。
//! 胶水留在 `src/delegate.rs`。
//!
//! 子 agent 不能 `OnceLock` 自登记：要 Provider 和子集，那些是运行时的。
//!
//! | 根 | 子 agent |
//! |---|---|
//! | LogPrefix `""` | `"  ↳ "` |
//! | Quiet false | true |
//! | Confirm 问你 | nil（自动过） |
//! | Name `""` | `"research"` |
//!
//! 本课不写 CodeReview（课上练习）、不写并行、不上第 12 课 TUI。
//!
//! 启动：`bash scripts/run.sh 11`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::agent::Agent;
    use build_rust_mini_coding_agent::api::{Message, ToolDef};
    use build_rust_mini_coding_agent::commands::{run_command, CommandCtx, CommandOutcome};
    use build_rust_mini_coding_agent::compact::NoCompaction;
    use build_rust_mini_coding_agent::delegate::{register_subagents, DelegateTool};
    use build_rust_mini_coding_agent::provider::MockProvider;
    use build_rust_mini_coding_agent::subagent::{Research, Subagent};
    use build_rust_mini_coding_agent::tools::{default_registry, Registry, Tool};

    struct FakeResearch;

    impl Subagent for FakeResearch {
        fn name(&self) -> &str {
            "research"
        }
        fn description(&self) -> String {
            "investigate".into()
        }
        fn run(&self, task: &str) -> Result<String, String> {
            Ok(format!("found:{task}"))
        }
    }

    #[test]
    fn lesson_11_research_subset_is_read_only() {
        let subset = default_registry().subset(&["read_file"]);
        let names: Vec<String> = subset.definitions().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["read_file".to_string()]);
        let (text, is_err) = subset.execute("write_file", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }

    #[test]
    fn lesson_11_delegate_name_and_result() {
        let tool = DelegateTool {
            subagent: std::sync::Arc::new(FakeResearch),
        };
        let def: ToolDef = tool.definition();
        assert_eq!(def.name, "delegate_research");
        let (text, is_err) = tool.execute(r#"{"task":"where is the loop"}"#);
        assert!(!is_err);
        assert_eq!(text, "found:where is the loop");
    }

    #[test]
    fn lesson_11_research_run_is_own_window() {
        let llm = MockProvider::text("loop is in src/agent.rs");
        let research = Research {
            provider: llm,
            tools: default_registry().subset(&["read_file"]),
        };
        let answer = research.run("locate the agent loop").unwrap();
        assert!(answer.contains("agent.rs"), "{answer}");
    }

    #[test]
    fn lesson_11_register_adds_delegate_to_tools() {
        let llm = MockProvider::text("x");
        let mut tools = default_registry().clone();
        let subagents = register_subagents(&llm, &mut tools);
        let names: Vec<String> = tools.definitions().into_iter().map(|d| d.name).collect();
        assert!(
            names.contains(&"delegate_research".to_string()),
            "{names:?}"
        );
        assert!(names.contains(&"read_file".to_string()));
        let listed: Vec<String> = subagents
            .all()
            .iter()
            .map(|s| s.name().to_string())
            .collect();
        assert_eq!(listed, vec!["research".to_string()]);
    }

    #[test]
    fn lesson_11_slash_subagents_lists_registry() {
        let mut llm = MockProvider::text("x");
        let mut tools = default_registry().clone();
        let subagents = register_subagents(&llm, &mut tools);
        let defs = tools.definitions();
        let compact = NoCompaction;
        let mut verbose = false;
        let mut messages: Vec<Message> = Vec::new();
        let mut ctx = CommandCtx {
            llm: &mut llm,
            messages: &mut messages,
            tools: &defs,
            compact: &compact,
            verbose: &mut verbose,
            subagents: &subagents,
        };
        assert_eq!(
            run_command("/subagents", &mut ctx),
            Some(CommandOutcome::Handled)
        );
        assert!(defs.iter().any(|d| d.name == "delegate_research"));
    }

    #[test]
    fn lesson_11_root_agent_send_appends_user() {
        let llm = MockProvider::text("ok");
        let mut agent = Agent::new(llm, String::new(), Registry::new());
        let text = agent.send("hello").unwrap();
        assert_eq!(text, "ok");
        assert_eq!(agent.messages[0].content[0].text, "hello");
    }
}
