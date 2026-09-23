//! 外层 REPL。总体和第 02 课 `use_gate = true`；第 01 课 demo 为 false。
//! 第 02 课：REPL 与 confirm 共用这一条 `lines`。
//! 第 03 课：变量叫 `llm`，类型是 `Provider`。换这一行就换供应商。

use std::io::{self, BufRead, Write};

use crate::agent::agent_loop;
use crate::api::Message;
use crate::provider::{DeepSeekProvider, Provider};
use crate::tools::default_tool_defs;

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
pub fn run_repl_with(llm: &mut dyn Provider, tools: &[crate::api::ToolDef], use_gate: bool) {
    let mut messages: Vec<Message> = Vec::new();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    loop {
        print!("> ");
        let _ = stdout.flush();
        let Some(Ok(line)) = lines.next() else {
            return;
        };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        messages.push(Message::user_text(input));
        messages = agent_loop(llm, tools, messages, &mut lines, use_gate);
    }
}
