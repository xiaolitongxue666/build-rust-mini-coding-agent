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
//! 第 07 课：`/compact` 和 `/verbose` 用来当场试策略，不用重启。
//! 第 10 课：命令碰到所有扩展点。搬进独立包就要把状态全传出去，或做成全局。留在集成层。
//! 第 11 课：`/subagents` 看登记和 `Active()`。REPL 堵住时飞行中的是空的。

use std::sync::OnceLock;

use crate::api::{Message, ToolDef};
use crate::compact::{
    print_compaction, CompactionStrategy, NoCompaction, SlidingWindow, Summarize,
};
use crate::provider::Provider;
use crate::subagent;

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
    pub compact: &'a dyn CompactionStrategy,
    pub verbose: &'a mut bool,
    pub subagents: &'a subagent::Registry,
}

/// 旧课单测没有子 agent。
pub fn no_subagents() -> &'static subagent::Registry {
    static EMPTY: OnceLock<subagent::Registry> = OnceLock::new();
    EMPTY.get_or_init(subagent::Registry::new)
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
            "compact",
            Command {
                description: "run compaction now (optionally with a specific strategy)",
                usage: "/compact [sliding|summarize|none]",
                run: cmd_compact,
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
            "subagents",
            Command {
                description: "list subagents (registered and currently running)",
                usage: "/subagents",
                run: cmd_subagents,
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
        (
            "verbose",
            Command {
                description: "toggle printing of compaction before/after",
                usage: "/verbose [on|off]",
                run: cmd_verbose,
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

/// 第 05 课 / 第 06 课：`messages.clear()` 就是清对话。
/// 模型没有记忆；下一轮 `send` 只带 Provider 上的 system。
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

fn cmd_subagents(_args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    let all = ctx.subagents.all();
    if all.is_empty() {
        println!("no subagents registered");
    } else {
        println!("registered subagents:");
        for sa in all {
            println!("  {:<16} {}", sa.name(), sa.description());
        }
    }
    let running = subagent::active();
    if running.is_empty() {
        println!("currently running: (none)");
    } else {
        println!("currently running:");
        let mut names: Vec<_> = running.into_iter().collect();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, n) in names {
            if n == 1 {
                println!("  {name}");
            } else {
                println!("  {name} ×{n}");
            }
        }
    }
    CommandOutcome::Handled
}

fn cmd_compact(args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    let sliding = SlidingWindow { keep_last: 6 };
    let summarize = Summarize {
        threshold: 0,
        keep_recent: 4,
        instructions: String::new(),
    };
    let none = NoCompaction;
    let strategy: &dyn CompactionStrategy = match args.to_ascii_lowercase().as_str() {
        "" => ctx.compact,
        "sliding" => &sliding,
        "summarize" => &summarize,
        "none" => &none,
        other => {
            println!("unknown strategy: {other} (try sliding, summarize, or none)");
            return CommandOutcome::Handled;
        }
    };

    let before = ctx.messages.clone();
    match strategy.compact(&before, ctx.llm) {
        Ok(after) => {
            println!("compacted: {} → {} messages", before.len(), after.len());
            if *ctx.verbose && before.len() != after.len() {
                print_compaction(&before, &after);
            }
            *ctx.messages = after;
        }
        Err(err) => println!("compaction error: {err}"),
    }
    CommandOutcome::Handled
}

fn cmd_verbose(args: &str, ctx: &mut CommandCtx<'_>) -> CommandOutcome {
    match args.to_ascii_lowercase().as_str() {
        "" => *ctx.verbose = !*ctx.verbose,
        "on" | "true" | "yes" => *ctx.verbose = true,
        "off" | "false" | "no" => *ctx.verbose = false,
        other => {
            println!("unknown value: {other} (try on/off)");
            return CommandOutcome::Handled;
        }
    }
    let state = if *ctx.verbose { "on" } else { "off" };
    println!("verbose: {state}");
    CommandOutcome::Handled
}
