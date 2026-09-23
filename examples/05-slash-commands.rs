//! 第 05 课：[Slash commands](https://www.byoharness.dev/chapters/05-slash-commands.html)
//!
//! 本课小 demo：以 `/` 开头的行在进模型之前拦截。实现在 crate 库里一份。
//!
//! ```text
//! [读一行]
//!     │
//!     ▼
//! [以 / 开头?]─no──▶ [append user，进 agent_loop]
//!     │
//!    yes
//!     │
//!     ▼
//! [登记表查名字]
//!     │
//!     ├─ 未知 ──▶ 打印 unknown，不送给模型
//!     ├─ /exit ──▶ 与 Ctrl+C 两次 / Ctrl+D 相同：Quit
//!     ├─ /clear ──▶ messages.clear()（对话框历史，不是输入框）
//!     └─ 其它 ──▶ 跑 handler，回到 >
//! ```
//!
//! | 命令 | 作用 |
//! |---|---|
//! | `/help` | 按名字列出命令 |
//! | `/model` / `/model name` | 看或改模型；不校验 id |
//! | `/clear` | 清对话。模型无状态，清本地切片就是清记忆 |
//! | `/tools` | 列出当前工具面 |
//! | `/exit` | 静默离开，与快捷键退出同一条路 |
//!
//! `SplitN(..., 2)`：`/model deepseek-chat` 的参数里可以有空格。
//! 课上用包级全局；本仓库用 `CommandCtx`，方便单测。
//!
//! 启动：`bash scripts/run.sh 05`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::api::Message;
    use build_rust_mini_coding_agent::commands::{
        clear_conversation, run_command, CommandCtx, CommandOutcome,
    };
    use build_rust_mini_coding_agent::provider::{MockProvider, Provider};
    use build_rust_mini_coding_agent::tools::default_tool_defs;

    fn run(
        line: &str,
        llm: &mut MockProvider,
        messages: &mut Vec<Message>,
    ) -> Option<CommandOutcome> {
        let tools = default_tool_defs();
        let compact = build_rust_mini_coding_agent::compact::NoCompaction;
        let mut verbose = false;
        let mut ctx = CommandCtx {
            llm,
            messages,
            tools: &tools,
            compact: &compact,
            verbose: &mut verbose,
        };
        run_command(line, &mut ctx)
    }

    #[test]
    fn lesson_05_plain_line_is_not_a_command() {
        let mut llm = MockProvider::text("x");
        let mut messages = Vec::new();
        assert_eq!(run("hello", &mut llm, &mut messages), None);
    }

    #[test]
    fn lesson_05_unknown_slash_is_handled() {
        let mut llm = MockProvider::text("x");
        let mut messages = Vec::new();
        assert_eq!(
            run("/asdf", &mut llm, &mut messages),
            Some(CommandOutcome::Handled)
        );
    }

    #[test]
    fn lesson_05_exit_matches_keyboard_quit() {
        let mut llm = MockProvider::text("x");
        let mut messages = Vec::new();
        assert_eq!(
            run("/exit", &mut llm, &mut messages),
            Some(CommandOutcome::Quit)
        );
    }

    #[test]
    fn lesson_05_clear_empties_dialog() {
        let mut llm = MockProvider::text("x");
        let mut messages = vec![Message::user_text("hi")];
        assert_eq!(
            run("/clear", &mut llm, &mut messages),
            Some(CommandOutcome::Handled)
        );
        assert!(messages.is_empty());
        messages.push(Message::user_text("again"));
        clear_conversation(&mut messages);
        assert!(messages.is_empty());
    }

    #[test]
    fn lesson_05_model_sets_without_validating() {
        let mut llm = MockProvider::text("x");
        let mut messages = Vec::new();
        assert_eq!(
            run("/model deepseek-chat", &mut llm, &mut messages),
            Some(CommandOutcome::Handled)
        );
        assert_eq!(llm.model(), "deepseek-chat");
    }
}
