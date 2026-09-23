//! 外层 REPL。总体和第 02 课 `use_gate = true`；第 01 课 demo 为 false。
//! 第 02 课：REPL 与 confirm 共用同一条输入。
//! 第 03 课：变量叫 `llm`，类型是 `Provider`。换这一行就换供应商。
//! 第 04 课：启动打 banner；读行走 SessionInput。
//! 第 08 课：一次性边框输入。第 12 课：TTY 整屏程序；管道仍走 stdin.lines()。
//! 第 06 课：`messages` 切片是对话的唯一真相来源。
//! 第 07 课：每轮进循环前跑压缩策略；默认 `NoCompaction`。
//! 第 09 课：工具面来自 `default_registry().definitions()`。
//! 第 10 课：REPL 是接线层，对齐课上 `main.go` 里那层循环。
//! 第 11 课：这里 `register_subagents`，再 new 根 `Agent`。DelegateTool 不进 tools 模块。
//! 第 12 课：TTY 走 `run_tui`；`BYO_PLAIN_INPUT` / 非 TTY 仍是同步 REPL。
//! 第 14 课：`mcp::setup` 在子 agent 和 `run_tui` 之前。连接错误打在真终端上。
//! 第 15 课：`AGENTS.md` 在 clone 子 agent 之前焊上。`Agent::new` 传入的是行为半边，不是拼好的全文。
//! 第 16 课：状态栏闭包和 provider 共享同一份账本。

use std::io::{self, BufRead, IsTerminal};
use std::sync::Arc;

use crate::agent::Agent;
use crate::chat::system_prompt;
use crate::commands::{run_command, CommandCtx, CommandOutcome};
use crate::compact::NoCompaction;
use crate::delegate::register_subagents;
use crate::provider::{DeepSeekProvider, Provider};
use crate::subagent;
use crate::tools::default_registry;
use crate::ui::{print_banner, run_tui, PromptRead, ReplLine};

pub fn run_repl(use_gate: bool) {
    let mut llm = match DeepSeekProvider::from_env() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    // 第 15 课：先焊上下文，再 clone 给 Research。缺文件就是空串。
    llm.attach_project_context(crate::agents_context::load_agents_context());
    let ledger = llm.usage_ledger();
    let usage_line = Arc::new(move || {
        let report = ledger
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .report();
        crate::pricing::usage_status(&report)
    });

    eprintln!(
        "model={} endpoint={} key_len={} gate={use_gate}",
        llm.model(),
        llm.endpoint(),
        llm.api_key_len()
    );

    let mut tools = default_registry().clone();
    // 第 14 课：子 agent 的 Subset 和 TUI 管子都还没开始。mcp.json 没有就不打日志。
    let mcp_clients = crate::mcp::setup(&mut tools);
    let subagents = register_subagents(&llm, &mut tools);
    // 行为半边。set_system 会把已经焊上的 AGENTS.md 再拼回去，不会贴两遍。
    let system = system_prompt(llm.model());
    let mut agent = Agent::new(llm, system, tools);
    agent.use_gate = use_gate;
    agent.max_turns = 50;
    let status = if use_plain_input() {
        run_plain(&mut agent, &subagents);
        0
    } else {
        match run_tui(agent, subagents, usage_line) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        }
    };
    // 第 14 课：退出时关掉 server。stdio 子进程不能留在后台。
    crate::mcp::close_all(&mcp_clients);
    if status != 0 {
        std::process::exit(status);
    }
}

/// 第 03 课：循环入口只认 trait。单测塞 `MockProvider`，live 塞 `DeepSeekProvider`。
pub fn run_repl_with<P>(llm: &mut P, tools: &[crate::api::ToolDef], use_gate: bool)
where
    P: Provider + Clone + Send + 'static,
{
    let registry = if tools.is_empty() {
        crate::tools::Registry::new()
    } else {
        default_registry().clone()
    };
    let mut agent = Agent::new(llm.clone(), String::new(), registry);
    agent.use_gate = use_gate;
    let empty = subagent::Registry::new();
    run_plain(&mut agent, &empty);
}

fn use_plain_input() -> bool {
    matches!(std::env::var("BYO_PLAIN_INPUT").ok().as_deref(), Some("1"))
        || !io::stdin().is_terminal()
}

fn run_plain<P>(agent: &mut Agent<P>, subagents: &subagent::Registry)
where
    P: Provider + Clone + Send + 'static,
{
    print_banner();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    run_turns(agent, subagents, &mut lines);
}

fn run_turns<P, R>(agent: &mut Agent<P>, subagents: &subagent::Registry, input: &mut R)
where
    P: Provider + Clone + Send + 'static,
    R: PromptRead + ReplSource,
{
    // 第 07 课：换这一行就换压缩策略。
    let compact = NoCompaction;
    let mut verbose = false;
    loop {
        match input.read_repl() {
            // /exit、Ctrl+C 两次、Ctrl+D 都走这里，不打 Error。
            ReplLine::Quit => return,
            ReplLine::Text(text) => {
                agent.verbose = verbose;
                let defs = agent.tools.definitions();
                let mut ctx = CommandCtx {
                    llm: &mut agent.llm,
                    messages: &mut agent.messages,
                    tools: &defs,
                    compact: &compact,
                    verbose: &mut verbose,
                    subagents,
                };
                match run_command(&text, &mut ctx) {
                    Some(CommandOutcome::Quit) => return,
                    Some(CommandOutcome::Handled) => continue,
                    None => {}
                }
                // 第 06 课 / 第 11 课：`Send` 自己 append user。
                if let Err(err) = agent.send_with(text, input) {
                    println!("{err}");
                }
            }
        }
    }
}

/// 管道把下一行当提交。
trait ReplSource {
    fn read_repl(&mut self) -> ReplLine;
}

impl<I> ReplSource for I
where
    I: Iterator<Item = std::io::Result<String>>,
{
    fn read_repl(&mut self) -> ReplLine {
        loop {
            match self.next() {
                Some(Ok(line)) => {
                    let input = line.trim();
                    if input.is_empty() {
                        continue;
                    }
                    return ReplLine::Text(input.to_string());
                }
                _ => return ReplLine::Quit,
            }
        }
    }
}
