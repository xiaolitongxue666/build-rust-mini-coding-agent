//! 第 02 课：harness 层权限门。审批夹在 `[tool]` 打印和分发之间。

use std::io::{self, Write};

use crate::tools::dispatch_tool;

/// 第 02 课：只认 `y` / `yes`。空行、EOF、其它字符都是否。
/// REPL 和这里必须共用同一条 `lines`，再 lock stdin 会抢字节。
pub fn confirm(prompt: &str, lines: &mut impl Iterator<Item = io::Result<String>>) -> bool {
    print!("{prompt} [y/n] ");
    let _ = io::stdout().flush();
    match lines.next() {
        Some(Ok(line)) => {
            let answer = line.trim().to_ascii_lowercase();
            answer == "y" || answer == "yes"
        }
        _ => false,
    }
}

/// 第 02 课：拒绝是 `("user denied this tool call", true)`，与文件不存在同一条合同。
pub fn execute_gated(
    name: &str,
    raw_input: &str,
    lines: &mut impl Iterator<Item = io::Result<String>>,
) -> (String, bool) {
    println!("[tool] {name} {raw_input}");
    if !confirm("approve?", lines) {
        return ("user denied this tool call".to_string(), true);
    }
    dispatch_tool(name, raw_input)
}

/// 第 01 课形状：打印 `[tool]` 后立刻分发，没有审批。
pub fn execute_direct(name: &str, raw_input: &str) -> (String, bool) {
    println!("[tool] {name} {raw_input}");
    dispatch_tool(name, raw_input)
}
