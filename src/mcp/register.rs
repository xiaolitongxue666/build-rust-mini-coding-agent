//! 第 14 课：把 MCP server 上的工具登记进本地 Registry。
//!
//! 失败的 server 打到 stderr 然后跳过。少一个工具不是致命错误。
//! 配置文件不存在连日志都不打——MCP 是 opt-in。
//!
//! `tools/list` 只在启动时打一次，串行。课上说生产可以并发，学习项目接受这点延迟。
//! 改 `mcp.json` 要重启。没有热重载。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::api::ToolDef;
use crate::mcp::client::{Client, ClientInner};
use crate::tools::{Registry, Tool};

use std::sync::Arc;

/// 课上的 `mcp.json`。相对**当前工作目录**，不是相对二进制。
pub const CONFIG_PATH: &str = "mcp.json";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub name: String,
    /// `"stdio"` 或 `"http"`。WebSocket 课上标了少见，这里不认。
    pub transport: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

/// 缺文件是 `Ok(None)`。解析失败才是 `Err`。
pub fn load_config(path: impl AsRef<Path>) -> Result<Option<Config>, String> {
    let path = path.as_ref();
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|err| format!("parse mcp config {}: {err}", path.display()))
}

/// 总体入口。读 cwd 下的 `mcp.json`。
pub fn setup(registry: &mut Registry) -> Vec<Client> {
    setup_at(Path::new(CONFIG_PATH), registry)
}

pub fn setup_at(path: impl AsRef<Path>, registry: &mut Registry) -> Vec<Client> {
    match load_config(path) {
        Ok(Some(config)) => register(&config, registry),
        Ok(None) => Vec::new(),
        Err(err) => {
            eprintln!("mcp: config error: {err}");
            Vec::new()
        }
    }
}

pub fn register(config: &Config, registry: &mut Registry) -> Vec<Client> {
    let mut clients = Vec::new();
    for server in &config.servers {
        let client = match dial(server) {
            Ok(client) => client,
            Err(err) => {
                eprintln!("mcp: skip server {:?}: {err}", server.name);
                continue;
            }
        };
        let listed = match client.list_tools() {
            Ok(listed) => listed,
            Err(err) => {
                eprintln!("mcp: list tools from {:?} failed: {err}", server.name);
                client.close();
                continue;
            }
        };
        let inner = client.shared();
        for remote in listed {
            let remote_name = remote.name.to_string();
            let Some((properties, required)) = split_schema(remote.input_schema.as_ref()) else {
                eprintln!(
                    "mcp: skip {}/{}: unrecognized input schema",
                    server.name, remote_name
                );
                continue;
            };
            registry.register(McpTool {
                client: Arc::clone(&inner),
                remote_name,
                def: ToolDef {
                    // 两个 server 都可能叫 read_file。前缀后扁平名单才不会互相盖掉。
                    name: exposed_name(&server.name, remote.name.as_ref()),
                    description: remote
                        .description
                        .as_ref()
                        .map(|text| text.to_string())
                        .unwrap_or_default(),
                    input_schema: properties,
                    required,
                },
            });
        }
        clients.push(client);
    }
    clients
}

pub fn close_all(clients: &[Client]) {
    for client in clients {
        client.close();
    }
}

pub fn exposed_name(server: &str, remote: &str) -> String {
    format!("{server}_{remote}")
}

/// 对齐 Go `os.ExpandEnv` 里本课用到的部分：`${VAR}`、`$VAR`、`$$`。
/// 未设置的变量换成空串。密钥留在环境里，不写进可能被提交的文件。
pub fn expand_env(input: &str) -> String {
    expand_with(input, |name| std::env::var(name).unwrap_or_default())
}

fn expand_with(input: &str, mut lookup: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        rest = &rest[dollar + 1..];
        if rest.is_empty() {
            out.push('$');
            break;
        }
        if let Some(after) = rest.strip_prefix('$') {
            out.push('$');
            rest = after;
            continue;
        }
        if let Some(body) = rest.strip_prefix('{') {
            let Some(end) = body.find('}') else {
                out.push('$');
                out.push_str(rest);
                break;
            };
            let name = &body[..end];
            if !name.is_empty() {
                out.push_str(&lookup(name));
            }
            rest = &body[end + 1..];
            continue;
        }
        let name_len = env_name_len(rest);
        if name_len == 0 {
            out.push('$');
            let mut chars = rest.chars();
            out.push(chars.next().expect("rest is non-empty"));
            rest = chars.as_str();
            continue;
        }
        let name = &rest[..name_len];
        out.push_str(&lookup(name));
        rest = &rest[name_len..];
    }
    out.push_str(rest);
    out
}

fn env_name_len(input: &str) -> usize {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return 0;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return 0;
    }
    let mut len = first.len_utf8();
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            break;
        }
        len += ch.len_utf8();
    }
    len
}

fn dial(server: &ServerConfig) -> Result<Client, String> {
    match server.transport.as_str() {
        "stdio" => {
            if server.command.is_empty() {
                return Err(format!("stdio server {:?} missing command", server.name));
            }
            let args: Vec<String> = server.args.iter().map(|arg| expand_env(arg)).collect();
            Client::connect_stdio(&server.name, &expand_env(&server.command), &args)
        }
        "http" => {
            if server.url.is_empty() {
                return Err(format!("http server {:?} missing url", server.name));
            }
            let headers = server
                .headers
                .iter()
                .map(|(key, value)| (key.clone(), expand_env(value)))
                .collect();
            Client::connect_http(&server.name, &expand_env(&server.url), &headers)
        }
        other => Err(format!("unknown transport {other:?} (want stdio or http)")),
    }
}

/// `$ref` 和认不出的 schema 跳过这一个工具，不跳过整个 server。
/// DeepSeek 的 function.parameters 不吃 JSON Schema 的 `$ref`。
fn split_schema(schema: &Map<String, Value>) -> Option<(Map<String, Value>, Vec<String>)> {
    if schema_uses_ref(&Value::Object(schema.clone())) {
        return None;
    }
    let properties = match schema.get("properties") {
        Some(Value::Object(properties)) => properties.clone(),
        Some(_) => return None,
        None => Map::new(),
    };
    let required = match schema.get("required") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(_) => return None,
        None => Vec::new(),
    };
    Some((properties, required))
}

fn schema_uses_ref(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.contains_key("$ref") || map.values().any(schema_uses_ref),
        Value::Array(items) => items.iter().any(schema_uses_ref),
        _ => false,
    }
}

struct McpTool {
    client: Arc<ClientInner>,
    /// server 上的原名。Registry 里是 `server_tool`，RPC 仍用原名。
    remote_name: String,
    def: ToolDef,
}

impl Tool for McpTool {
    fn definition(&self) -> ToolDef {
        self.def.clone()
    }

    fn execute(&self, input: &str) -> (String, bool) {
        // 模型给的是一段 JSON。MCP 要对象。空串当成没有参数。
        let arguments = if input.is_empty() {
            None
        } else {
            match serde_json::from_str::<Value>(input) {
                Ok(Value::Object(map)) => Some(map),
                Ok(_) => {
                    return ("invalid tool input: expected JSON object".to_string(), true);
                }
                Err(err) => return (format!("invalid tool input: {err}"), true),
            }
        };
        match self.client.call_tool(&self.remote_name, arguments) {
            Ok(pair) => pair,
            Err(err) => (err, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_is_opt_in() {
        let path = std::env::temp_dir().join("byo-mcp-missing-config-does-not-exist.json");
        let _ = fs::remove_file(&path);
        assert_eq!(load_config(&path).unwrap(), None);
    }

    #[test]
    fn bad_json_is_config_error() {
        let path = std::env::temp_dir().join("byo-mcp-bad.json");
        fs::write(&path, b"{").unwrap();
        let err = load_config(&path).unwrap_err();
        let _ = fs::remove_file(&path);
        assert!(err.contains("parse mcp config"), "{err}");
    }

    #[test]
    fn expand_env_refs_and_dollar() {
        let text = expand_with("Bearer ${TOKEN} $TOKEN $$ tail", |name| {
            assert_eq!(name, "TOKEN");
            "abc".to_string()
        });
        assert_eq!(text, "Bearer abc abc $ tail");
        let missing = expand_with("${NO_SUCH_BYO_VAR}", |_| String::new());
        assert_eq!(missing, "");
    }

    #[test]
    fn ref_schema_skips_one_tool() {
        let ok = Map::from_iter([(
            "properties".to_string(),
            serde_json::json!({"tz": {"type": "string"}}),
        )]);
        assert!(split_schema(&ok).is_some());
        let broken = Map::from_iter([("$ref".to_string(), Value::String("#/defs/X".into()))]);
        assert!(split_schema(&broken).is_none());
    }

    #[test]
    fn unknown_transport_skips_server() {
        let mut registry = Registry::new();
        let config = Config {
            servers: vec![ServerConfig {
                name: "nope".to_string(),
                transport: "websocket".to_string(),
                command: String::new(),
                args: Vec::new(),
                url: String::new(),
                headers: HashMap::new(),
            }],
        };
        let clients = register(&config, &mut registry);
        assert!(clients.is_empty());
        assert!(registry.definitions().is_empty());
    }

    #[test]
    fn exposed_names_do_not_collide() {
        assert_eq!(exposed_name("git", "status"), "git_status");
        assert_eq!(
            exposed_name("filesystem", "read_file"),
            "filesystem_read_file"
        );
        assert_ne!(
            exposed_name("git", "read_file"),
            exposed_name("filesystem", "read_file")
        );
    }
}
