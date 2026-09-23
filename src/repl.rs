//! 外层 REPL。总体和第 02 课 `use_gate = true`；第 01 课 demo 为 false。
//! 第 02 课：REPL 与 confirm 共用同一条输入。
//! 第 03 课：变量叫 `llm`，类型是 `Provider`。换这一行就换供应商。
//! 第 04 课：启动打 banner；读行走 SessionInput。

use std::io::{self, BufRead, IsTerminal};

use crate::agent::agent_loop;
use crate::api::Message;
use crate::provider::{DeepSeekProvider, Provider};
use crate::tools::default_tool_defs;
use crate::ui::{print_banner, PromptRead, ReplLine, SessionInput};

pub fn run_repl(use_gate: bool) {
    let mut llm = match DeepSeekProvider::from_env() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "model={} endpoint={} key_len={} gate={use_gate}",
        llm.model(),
        llm.endpoint(),
        llm.api_key_len()
    );

    run_repl_with(&mut llm, &default_tool_defs(), use_gate);
}

/// 第 03 课：循环入口只认 trait。单测塞 `MockProvider`，live 塞 `DeepSeekProvider`。
pub fn run_repl_with<P>(llm: &mut P, tools: &[crate::api::ToolDef], use_gate: bool)
where
    P: Provider + Clone + Send + 'static,
{
    print_banner();

    // 管道 / BYO_PLAIN_INPUT=1：rustyline 在 Windows 上仍会 Ok，行进不了 editor。
    if use_plain_input() {
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        run_session(llm, tools, use_gate, &mut lines);
        return;
    }

    match SessionInput::new() {
        Ok(mut session) => run_session(llm, tools, use_gate, &mut session),
        Err(_) => {
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
            run_session(llm, tools, use_gate, &mut lines);
        }
    }
}

fn use_plain_input() -> bool {
    matches!(std::env::var("BYO_PLAIN_INPUT").ok().as_deref(), Some("1"))
        || !io::stdin().is_terminal()
}

fn run_session<P, R>(llm: &mut P, tools: &[crate::api::ToolDef], use_gate: bool, input: &mut R)
where
    P: Provider + Clone + Send + 'static,
    R: PromptRead + ReplSource,
{
    let mut messages: Vec<Message> = Vec::new();
    loop {
        match input.read_repl() {
            ReplLine::Quit => return,
            ReplLine::Text(text) => {
                messages.push(Message::user_text(text));
                messages = agent_loop(llm, tools, messages, input, use_gate);
            }
        }
    }
}

/// 空闲提示行。SessionInput 自己处理 Ctrl+C；迭代器回退把下一行当提交。
trait ReplSource {
    fn read_repl(&mut self) -> ReplLine;
}

impl ReplSource for SessionInput {
    fn read_repl(&mut self) -> ReplLine {
        SessionInput::read_repl(self)
    }
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
