//! 第 01 课：按名字分发三个工具。返回 `(正文, is_error)`，不是 Rust `Result`。
//! 第 03 课：工具面改成通用 `ToolDef`；OpenAI 那层 JSON 由 DeepSeek 适配器翻译。

use std::fs;
use std::process::Command;

use serde_json::{json, Map, Value};

use crate::api::ToolDef;

/// 第 01 课：发给模型的工具面。拆出 read/write 是为了可拦截。第 09 课才变 Registry。
pub fn default_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "bash".to_string(),
            description: "Run a shell command and return its combined stdout/stderr.".to_string(),
            input_schema: string_prop(&[("command", "The command to run.")]),
            required: vec!["command".to_string()],
        },
        ToolDef {
            name: "read_file".to_string(),
            description: "Read the contents of a file at the given path.".to_string(),
            input_schema: string_prop(&[("path", "Filesystem path to read.")]),
            required: vec!["path".to_string()],
        },
        ToolDef {
            name: "write_file".to_string(),
            description: "Write content to a file (creating or overwriting it).".to_string(),
            input_schema: string_prop(&[
                ("path", "Filesystem path to write."),
                ("content", "The bytes to write."),
            ]),
            required: vec!["path".to_string(), "content".to_string()],
        },
    ]
}

fn string_prop(fields: &[(&str, &str)]) -> Map<String, Value> {
    let mut props = Map::new();
    for (name, description) in fields {
        props.insert(
            (*name).to_string(),
            json!({"type": "string", "description": description}),
        );
    }
    props
}

/// 第 01 课：幻觉工具名走 error result，不要 panic。第 09 课才变 Registry。
pub fn dispatch_tool(name: &str, raw_input: &str) -> (String, bool) {
    match name {
        "bash" => {
            let command = match json_field(raw_input, "command") {
                Ok(v) => v,
                Err(e) => return (e, true),
            };
            run_shell(&command)
        }
        "read_file" => {
            let path = match json_field(raw_input, "path") {
                Ok(v) => v,
                Err(e) => return (e, true),
            };
            match fs::read_to_string(&path) {
                Ok(data) => (data, false),
                Err(e) => (e.to_string(), true),
            }
        }
        "write_file" => {
            let path = match json_field(raw_input, "path") {
                Ok(v) => v,
                Err(e) => return (e, true),
            };
            let content = match json_field(raw_input, "content") {
                Ok(v) => v,
                Err(e) => return (e, true),
            };
            match fs::write(&path, content) {
                Ok(()) => (format!("wrote {path}"), false),
                Err(e) => (e.to_string(), true),
            }
        }
        other => (format!("unknown tool: {other}"), true),
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

pub fn json_field(raw: &str, key: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field {key}"))
}
