//! 第 02 课：harness 层权限门。审批夹在 `[tool]` 打印和分发之间。
//! 第 04 课：confirm 走 `PromptRead`，和 REPL 共用一把 editor。Esc 是 Abort，不是退出进程。
//! 第 09 课：分发交给 Registry；这里只负责打 `[tool]` 和审批。
//! 第 11 课：根/子 Agent 自己打 `{LogPrefix}[tool]` 并 `tools.execute`。这里留给旧课 demo。
//! 第 12 课：`PromptRead` 放这里，循环不再 import `ui`，避免 `agent → ui → subagent → agent`。

use std::io::{self, Write};

use crate::tools::dispatch_tool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineResult {
    Line(String),
    End,
    Abort,
}

pub trait PromptRead {
    fn read_line(&mut self, prompt: &str) -> LineResult;
}

impl<I> PromptRead for I
where
    I: Iterator<Item = io::Result<String>>,
{
    fn read_line(&mut self, prompt: &str) -> LineResult {
        if !prompt.is_empty() {
            print!("{prompt}");
            let _ = io::stdout().flush();
        }
        match self.next() {
            Some(Ok(line)) => LineResult::Line(line),
            _ => LineResult::End,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Yes,
    No,
    Abort,
}

pub fn confirm_decision(prompt: &str, input: &mut impl PromptRead) -> Decision {
    match input.read_line(&format!("{prompt} [y/n] ")) {
        LineResult::Line(line) => {
            let answer = line.trim().to_ascii_lowercase();
            if answer == "y" || answer == "yes" {
                Decision::Yes
            } else {
                Decision::No
            }
        }
        LineResult::End => Decision::No,
        LineResult::Abort => Decision::Abort,
    }
}

/// 第 02 课：只认 `y` / `yes`。空行、EOF、其它字符都是否。
pub fn confirm(prompt: &str, input: &mut impl PromptRead) -> bool {
    matches!(confirm_decision(prompt, input), Decision::Yes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Ran(String, bool),
    Denied,
    Aborted,
}

impl GateResult {
    pub fn into_pair(self) -> (String, bool) {
        match self {
            GateResult::Ran(text, is_err) => (text, is_err),
            GateResult::Denied | GateResult::Aborted => {
                ("user denied this tool call".to_string(), true)
            }
        }
    }
}

pub fn execute_gated_result(
    name: &str,
    raw_input: &str,
    input: &mut impl PromptRead,
) -> GateResult {
    println!("[tool] {name} {raw_input}");
    match confirm_decision("approve?", input) {
        Decision::Yes => {
            let (text, is_err) = dispatch_tool(name, raw_input);
            GateResult::Ran(text, is_err)
        }
        Decision::No => GateResult::Denied,
        Decision::Abort => GateResult::Aborted,
    }
}

/// 第 02 课：拒绝是 `("user denied this tool call", true)`，与文件不存在同一条合同。
pub fn execute_gated(name: &str, raw_input: &str, input: &mut impl PromptRead) -> (String, bool) {
    execute_gated_result(name, raw_input, input).into_pair()
}

/// 第 01 课形状：打印 `[tool]` 后立刻分发，没有审批。
pub fn execute_direct(name: &str, raw_input: &str) -> (String, bool) {
    println!("[tool] {name} {raw_input}");
    dispatch_tool(name, raw_input)
}
