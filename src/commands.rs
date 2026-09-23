//! 第 05 课：[Slash commands](https://www.byoharness.dev/chapters/05-slash-commands.html)
//!
//! 以 `/` 开头的行在进模型之前拦截。`run_command` 回 `true` 表示已经处理。
//! 未知命令也回 `true`，否则 `/asdf` 会送给模型。
//!
//! 课上把 provider / messages 提成包级全局。本仓库单测和双入口都要注入状态，
//! 命令层用 `CommandCtx`，不写 `static`。
//!
//! `/exit` 与 Ctrl+C 两次 / Ctrl+D 同一条 `CommandOutcome::Quit`。
//! `/clear` 清对话框（`messages`），不是清输入框（那是 Ctrl+C 一次）。

use crate::api::{Message, ToolDef};
use crate::provider::Provider;

pub const KNOWN_MODELS: &[&str] = &["deepseek-flash", "deepseek-chat", "deepseek-reasoner"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOutcome {
    /// 已处理，不要送给模型。
    Handled,
    /// 与快捷键退出相同：REPL `return`，不打 Error。
    Quit,
}

pub struct CommandCtx<'a> {
    pub llm: &'a mut dyn Provider,
    pub messages: &'a mut Vec<Message>,
    pub tools: &'a [ToolDef],
}

struct Command {
    description: &'static str,
    usage: &'static str,
    run: fn(&str, &mut CommandCtx<'_>) -> CommandOutcome,
}

fn registry() -> Vec<(&'static str, Command)> {
    vec![
        (
            "clear",
            Command {
                description: "clear conversation history",
                usage: "/clear",
                run: cmd_clear,
            },
        ),
        (
            "exit",
            Command {
                description: "exit the harness",
                usage: "/exit",
                run: cmd_exit,
            },
        ),
        (
            "help",
            Command {
                description: "show available commands",
                usage: "/help",
                run: cmd_help,
            },
        ),
        (
            "model",
            Command {
                description: "show or change the model",
                usage: "/model [name]",
                run: cmd_model,
            },
        ),
        (
            "tools",
            Command {
                description: "list available tools",
                usage: "/tools",
                run: cmd_tools,
            },
        ),
    ]
}

/// 非 `/` 回 `None`（送给模型）。`/` 一律 `Some`，包括未知命令。
pub fn run_command(line: &str, ctx: &mut CommandCtx<'_>) -> Option<CommandOutcome> {
    if !line.starts_with('/') {
        return None;
    }
    let rest = &line[1..];
    let (name, args) = match rest.split_once(' ') {
        Some((n, a)) => (n, a.trim()),
        None => (rest, ""),
    };
    let Some((_, cmd)) = registry().into_iter().find(|(n, _)| *n == name) else {
        println!("unknown command: /{name} (try /help)");
        return Some(CommandOutcome::Handled);
    };
    Some((cmd.run)(args, ctx))
}

/// 第 05 课：`messages.clear()` 就是清对话。模型无状态。
pub fn clear_conversation(messages: &mut Vec<Message>) {
    messages.clear();
}

fn cmd_clear(_args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    clear_conversation(ctx.messages);
    println!("conversation cleared");
    CommandOutcome::Handled
}

fn cmd_exit(_args: &str, _ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    CommandOutcome::Quit
}

fn cmd_help(_args: &str, _ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    let mut rows = registry();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    for (_, cmd) in rows {
        println!("  {:<22} {}", cmd.usage, cmd.description);
    }
    CommandOutcome::Handled
}

fn cmd_model(args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    if args.is_empty() {
        println!("current: {}", ctx.llm.model());
        println!("suggestions:");
        for model in KNOWN_MODELS {
            println!("  {model}");
        }
        return CommandOutcome::Handled;
    }
    // 第 05 课：不校验 id。错了由下一轮 API 报错。
    ctx.llm.set_model(args.to_string());
    println!("model: {args}");
    CommandOutcome::Handled
}

fn cmd_tools(_args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    for tool in ctx.tools {
        println!("  {:<16} {}", tool.name, tool.description);
    }
    CommandOutcome::Handled
}
