//! 第 14 课测试夹具：最小 stdio MCP server。
//!
//! 只说 initialize / tools/list / tools/call。不是 harness，也不进 `run.sh`。
//! 协议正文留给官方 SDK；这里把握手答完，好让客户端测试不依赖 uvx / npx。

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => return,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(err) => {
                eprintln!("mcp fixture: {err}");
                return;
            }
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = message.get("id").filter(|id| !id.is_null()) else {
            continue;
        };
        let result = match method {
            "initialize" => initialize_result(&message),
            "tools/list" => tools(),
            "tools/call" => call_tool(&message),
            _ => json!({}),
        };
        let reply = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        if writeln!(stdout, "{reply}").is_err() {
            return;
        }
        if stdout.flush().is_err() {
            return;
        }
    }
}

fn initialize_result(message: &Value) -> Value {
    let version = message
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("2025-11-25");
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "fixture", "version": "0.1" }
    })
}

fn tools() -> Value {
    json!({
        "tools": [
            {
                "name": "current_time",
                "description": "fixture clock",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tz": { "type": "string", "description": "timezone" }
                    },
                    "required": ["tz"]
                }
            },
            {
                "name": "blob",
                "description": "non-text block",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "boom",
                "description": "tool error",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "broken",
                "description": "schema the provider would reject",
                "inputSchema": { "$ref": "#/defs/X" }
            }
        ]
    })
}

fn call_tool(message: &Value) -> Value {
    let name = message
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or("");
    match name {
        "current_time" => {
            let tz = message
                .pointer("/params/arguments/tz")
                .and_then(Value::as_str)
                .unwrap_or("");
            json!({
                "content": [{ "type": "text", "text": format!("ok:{tz}") }],
                "isError": false
            })
        }
        "blob" => json!({
            "content": [{ "type": "image", "data": "YQ==", "mimeType": "image/png" }],
            "isError": false
        }),
        "boom" => json!({
            "content": [{ "type": "text", "text": "nope" }],
            "isError": true
        }),
        other => json!({
            "content": [{ "type": "text", "text": format!("unknown:{other}") }],
            "isError": true
        }),
    }
}
