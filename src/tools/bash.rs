//! 第 09 课：一个文件一个工具。登记在 `default_registry()`，不进 main。

use std::process::Command;

use crate::api::ToolDef;
use crate::tools::{json_field, string_prop, Tool};

pub struct BashTool;

impl Tool for BashTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "bash".to_string(),
            description: "Run a shell command and return its combined stdout/stderr.".to_string(),
            input_schema: string_prop(&[("command", "The command to run.")]),
            required: vec!["command".to_string()],
        }
    }

    fn execute(&self, raw_input: &str) -> (String, bool) {
        let command = match json_field(raw_input, "command") {
            Ok(v) => v,
            Err(e) => return (e, true),
        };
        run_shell(&command)
    }
}

/// 课程 Go 写死 `sh -c`。Windows 上优先 Git Bash，课程示例里的 `ls` / `pwd` 还能对上。
fn run_shell(command: &str) -> (String, bool) {
    let output = if has_cmd("bash") {
        Command::new("bash").arg("-lc").arg(command).output()
    } else if cfg!(windows) {
        Command::new("cmd").arg("/C").arg(command).output()
    } else {
        Command::new("sh").arg("-c").arg(command).output()
    };

    match output {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&stderr);
            }
            if out.status.success() {
                (text, false)
            } else {
                (format!("{text}\n[exit error: {}]", out.status), true)
            }
        }
        Err(e) => (e.to_string(), true),
    }
}

fn has_cmd(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
