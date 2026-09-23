//! 第 01 课：[The agent loop](https://www.byoharness.dev/chapters/01-the-agent-loop.html)
//!
//! 用 **Rust** 对照 Go 的 `examples/minimal/main.go`。一个文件就是完整 agent：
//! 外层 REPL + 内层 tool-use 循环 + `match` 分发三个工具。没有 Provider、没有
//! 权限门、没有 TUI。第 03–12 课才会一层层加回去；不要拿 HEAD 的 `src/main.rs`
//! 来对这一课的行号。
//!
//! 课程整张图就这一圈（模型决定做什么，harness 执行，直到模型不再要工具）：
//!
//! ```text
//! [你的输入]
//!     │
//!     ▼
//! [append 到 messages]
//!     │
//!     ▼
//! [调用模型] ─────────┐
//!     │               │
//!     ▼               │
//! [有 tool_calls?]─no─┴──▶ [打印文本，回到 REPL]
//!     │
//!    yes
//!     │
//!     ▼
//! [执行每个工具]
//!     │
//!     ▼
//! [append tool 结果]
//!     │
//!     ▼
//! (回到「调用模型」)
//! ```
//!
//! 上面是 **内层**（一轮 agent turn）。外面还包着 **REPL**（Read–Eval–Print–Loop）：
//! 游戏循环每秒 60 次按时钟走；REPL 按你的回车走。这里的 Eval 不是「跑一段代码」，
//! 而是「对你这行字跑一遍 agent 循环」。
//!
//! | 循环 | 谁在推 | 一轮是什么 |
//! |---|---|---|
//! | 外层 REPL | 你的键盘 | 读一行 → `agent_loop` → 等下一行 |
//! | 内层 agent | 模型的选择 | 调模型 → 若有 tool_calls 则执行并 append → 重复直到停 |
//!
//! 课程正文用 Anthropic Messages 词表。本仓库 live 走 DeepSeek 的 OpenAI 兼容
//! Chat Completions（`reqwest` 自己拼 JSON，不用官方 SDK）。**词换了，循环没换。**
//! 系统提示必须自称 DeepSeek 并带上当前模型名；模型看不见请求里的 `model=`。
//!
//! | 课程（Anthropic） | 本文件（DeepSeek / OpenAI） | 含义 |
//! |---|---|---|
//! | `messages` | `messages` | 到目前为止的全部对话；API 无状态，客户端带着走 |
//! | `tools` + `input_schema` | `tools` + `function.parameters` | 模型能调用的操作面（JSON Schema） |
//! | content 里的 `tool_use` | `message.tool_calls` | 模型要 harness 在本地跑某个工具 |
//! | `stop_reason: tool_use` | `finish_reason: tool_calls` | 内层继续转 |
//! | `stop_reason: end_turn` | `finish_reason: stop` | 打出文本，回到 REPL |
//! | `tool_result` + `tool_use_id` | `role: "tool"` + `tool_call_id` | 必须对上 id，对不上 API 400 |
//! | `max_tokens` | `max_tokens` | 限制 **输出** 长度，不是输入 |
//!
//! 启动：`bash scripts/run.sh`（脚本才加载密钥）。不要在这个文件里写 API key。

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process::Command;

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// 第 01 课：模型看不见请求里的 `model=`。只写 “coding assistant” 时，
// 训练数据里最常见的助手人设是 Claude，「你是谁」会答错。身份必须写进 system。
fn system_prompt(model: &str) -> String {
    format!(
        "You are DeepSeek, a coding assistant running in a terminal. \
         The model serving this session is {model} (DeepSeek, not Claude, not GPT). \
         You have three tools: bash, read_file, write_file. Be concise."
    )
}

const DEFAULT_MODEL: &str = "deepseek-flash";
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

// 第 01 课：只封输出。~4 个英文字符约 1 token。输入历史会越积越长，那是第 06 / 07 课的事。
const MAX_TOKENS: u32 = 8192;

/// 第 01 课：发给模型的工具面。
///
/// 可以只给一个 `bash`（它能读写文件）。拆出 `read_file` / `write_file` 不是因为
/// 做不到，而是因为它们 **可拦截**：后面的权限门、diff 审批都挂在具体工具名上。
/// 只留 bash 时，审批看到的是不透明命令串。工具面形状是 harness 决策，模型不在乎
/// 你给一个还是三个。
///
/// 课程 Go 用 Anthropic 的 `ToolParam`；这里是 Chat Completions 的
/// `{type:function, function:{name, parameters}}`。第 09 课才会变成 `Registry`。
fn tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "bash",
                "description": "Run a shell command and return its combined stdout/stderr.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "The command to run."}
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file at the given path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Filesystem path to read."}
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file (creating or overwriting it).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Filesystem path to write."},
                        "content": {"type": "string", "description": "The bytes to write."}
                    },
                    "required": ["path", "content"]
                }
            }
        }
    ])
}

/// 第 01 课：一条对话消息。API 无状态，客户端带着全部历史（第 06 课展开）。
///
/// Go 课用 SDK 的 `MessageParam` / `ContentBlock`。Rust 这边没有官方 SDK，
/// 用 `serde` 结构体直接对上线格式。`content` 用 `Value`：assistant 在
/// `tool_calls` 时经常给 JSON `null`，`Option<String>` 会解失败。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    // 只在 role=tool 时有。必须等于对应 tool_calls[].id，否则下一轮 400。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// 第 01 课：模型的「请帮我跑这个」请求。对应 Anthropic 的 `tool_use` block。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    // 已经是 JSON 字符串，不是对象。execute_tool 再 parse 一次。
    arguments: String,
}

/// Chat Completions 的外壳。有用的部分在 `choices[0]`。
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

/// `finish_reason` 驱动内层循环，对应课程的 `stop_reason`。
/// 不是 `tool_calls` 的（`stop` / `length` / …）一律当「本轮结束」。
#[derive(Debug, Deserialize)]
struct Choice {
    finish_reason: Option<String>,
    message: ChatMessage,
}

fn main() {
    // 密钥只从环境读。`scripts/run.sh` 才会去家目录加载；cargo run 裸跑会走到这里退出。
    let api_key = env::var("DEEPSEEK_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("缺少 DEEPSEEK_API_KEY。请用 bash scripts/run.sh 启动。");
        std::process::exit(1);
    }

    let model = env::var("LLM_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let base_url = env::var("OPENAI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let endpoint = chat_completions_url(&base_url);

    // Go 课：`anthropic.NewClient()` 读 ANTHROPIC_API_KEY。
    // Rust：blocking `reqwest`，和课程同步循环同一形状，还没上 tokio。
    // 不自动跟重定向：跳到另一路径时 reqwest 会丢掉 Authorization，官方会回 401。
    let http = Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("reqwest client");

    eprintln!(
        "model={model} endpoint={endpoint} key_len={}",
        api_key.len()
    );

    // 没有工具时，内层塌缩成聊天客户端：append → 调模型 → 打印 → 等下一行。
    // 加上 tools 之后，同一条 `messages` 会在内层被来回 append（assistant + tool）。
    // Anthropic 的 `system` 是请求顶栏字段；Chat Completions 把它做成第一条 message。
    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: Some(Value::String(system_prompt(&model))),
        tool_calls: None,
        tool_call_id: None,
    }];

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    // 第 01 课 · 外层 REPL。Go 用 `bufio.Scanner`；Rust 用 `stdin.lock().lines()`。
    // Print 发生在内层（模型文本、`[tool] ...`），所以这里看起来像 Read → Eval → 等下一行。
    loop {
        print!("> ");
        let _ = stdout.flush();
        let Some(Ok(line)) = lines.next() else {
            // EOF（Ctrl-D / Ctrl-Z）：结束外层，不是内层出错。
            return;
        };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        // 课程 11 步的第 1–2 步：读到一行，append 成 user 消息。
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(Value::String(input.to_string())),
            tool_calls: None,
            tool_call_id: None,
        });
        messages = agent_loop(&http, &endpoint, &api_key, &model, messages);
    }
}

/// 第 01 课 · 内层 agent 循环。精神上递归，代码上是 `loop`。
///
/// 两个出口（课程原话）：
/// - 模型返回文本 → 打印，把更新后的 `messages` 交还 REPL
/// - 模型返回 tool call → 执行、append 结果、再问一次
///
/// 你打一句 `list the files here`，内层可能转两圈（先 bash，再总结）。
/// 打一句「法国首都」可能只转一圈。圈数由模型选，harness 服从。
///
/// 课程 11 步对照（`list the files here`）：
/// 3. POST `{model, messages, tools}`
/// 4. 回来 `finish_reason=tool_calls`，带一个 bash
/// 5. append assistant（含 tool_calls）；打印 `[tool] bash ...`
///    （原课第 6 步 `approve?` 是第 02 课，本文件故意没有）
/// 7. 本地跑命令
/// 8. append `role=tool`，`tool_call_id` 对上第 4 步的 id
/// 9. 因为是 tool_calls，再 POST 一次
/// 10–11. `finish_reason=stop`，打印文本，返回 REPL
fn agent_loop(
    http: &Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    mut messages: Vec<ChatMessage>,
) -> Vec<ChatMessage> {
    loop {
        let resp = match call_model(http, endpoint, api_key, model, &messages) {
            Ok(r) => r,
            Err(err) => {
                println!("api error: {err}");
                return messages;
            }
        };

        let Some(choice) = resp.choices.into_iter().next() else {
            println!("api error: empty choices");
            return messages;
        };

        let assistant = choice.message;
        let finish = choice.finish_reason.unwrap_or_default();
        let tool_calls = assistant.tool_calls.clone().unwrap_or_default();

        // 同一条 assistant 里可以同时有一段话和 tool_calls（先解释再动手）。
        if let Some(Value::String(text)) = &assistant.content {
            if !text.is_empty() {
                println!("{text}");
            }
        }

        // 第 01 课陷阱：必须把 assistant **原样** append 回去。
        // Go 课是 `messages = append(messages, resp.ToParam())`。漏掉则：
        // 1) 下一轮模型不知道自己刚说了什么；
        // 2) 后面的 tool 消息对不上 id，API 报孤儿 tool result / 400。
        messages.push(assistant);

        // 第 01 课陷阱：退出条件写成 `finish == "stop"` 会漏掉 `length` 等，
        // 要么空转要么该停不停。可靠判断是「有没有 tool_calls」，
        // 等价于课程的 `stop_reason != tool_use`。
        if finish != "tool_calls" || tool_calls.is_empty() {
            return messages;
        }

        for call in tool_calls {
            let (result, is_err) = execute_tool(&call.function.name, &call.function.arguments);
            if is_err {
                eprintln!("[tool error] {}", truncate(&result, 200));
            }
            // Anthropic：`tool_result` 挂在下一条 **user** 消息的 content 里。
            // OpenAI / DeepSeek：独立的 `role=tool` 消息。循环形状一样，信封不同。
            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(Value::String(result)),
                tool_calls: None,
                tool_call_id: Some(call.id),
            });
        }
    }
}

/// POST Chat Completions。Go 课藏在 `client.Messages.New` 后面；这里把线格式摊开。
fn call_model(
    http: &Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<ChatResponse, String> {
    let body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": messages,
        "tools": tool_defs(),
    });

    let resp = http
        .post(endpoint)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let text = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status} {}", redact_api_error(&text)));
    }
    serde_json::from_str(&text).map_err(|e| format!("{e}: {}", redact_api_error(&text)))
}

/// 第 01 课：按名字分发工具。Go 用 `switch`；Rust 用 `match`，形状相同。
///
/// 返回 `(正文, 是否失败)`，**不是** `Result`。路径不存在时模型读到
/// `"no such file or directory"`，可以换路径或告诉你。这里若 `unwrap` 崩循环，
/// 模型没有恢复通道。第 02 课会把「用户拒绝」也做成同一条合同。
///
/// 开头的 `[tool] ...` 只是可观测性，不参与协议。
/// `other` 分支防模型幻觉工具名，让它自己改口，不要 panic。
fn execute_tool(name: &str, raw_input: &str) -> (String, bool) {
    println!("[tool] {name} {raw_input}");
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

/// 课程 Go 写死 `sh -c`。Windows 上 `sh` 经常不在 PATH，优先 Git Bash，
/// 这样课程示例里的 `ls` / `pwd` 还能对上。
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

fn json_field(raw: &str, key: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field {key}"))
}

/// DeepSeek 官方与 dsh 都是 `{base}/chat/completions`，不要默认再拼 `/v1`。
/// 见 https://api-docs.deepseek.com/ ；`/v1/...` 若 30x 到无 Authorization 的路径会 401。
fn chat_completions_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

/// 错误正文里可能回显密钥片段。只留 type/code，方便排查又不把 secret 打到终端。
fn redact_api_error(raw: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        let err = &value["error"];
        if err.is_object() {
            return format!(
                "type={} code={}",
                err["type"].as_str().unwrap_or("?"),
                err["code"].as_str().unwrap_or("?")
            );
        }
    }
    if raw.len() > 160 {
        format!("{}…", raw.chars().take(160).collect::<String>())
    } else {
        raw.to_string()
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(suffix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("byo-lesson01-{nanos}-{suffix}"))
    }

    #[test]
    fn chat_url_uses_official_completions_path() {
        assert_eq!(
            chat_completions_url("https://api.deepseek.com"),
            "https://api.deepseek.com/chat/completions"
        );
    }

    #[test]
    fn chat_url_keeps_explicit_completions_suffix() {
        assert_eq!(
            chat_completions_url("https://api.deepseek.com/chat/completions/"),
            "https://api.deepseek.com/chat/completions"
        );
    }

    #[test]
    fn redact_error_keeps_type_not_key() {
        let raw = r#"{"error":{"message":"Authentication Fails, Your api key: SECRET is invalid","type":"authentication_error","code":"invalid_request_error"}}"#;
        let out = redact_api_error(raw);
        assert!(out.contains("authentication_error"), "{out}");
        assert!(!out.contains("SECRET"), "{out}");
    }

    #[test]
    fn json_field_reads_string() {
        assert_eq!(
            json_field(r#"{"path":"Cargo.toml"}"#, "path").unwrap(),
            "Cargo.toml"
        );
    }

    #[test]
    fn json_field_missing_is_error_string() {
        let err = json_field(r#"{"path":"x"}"#, "command").unwrap_err();
        assert!(err.contains("command"), "{err}");
    }

    // 第 01 课：幻觉工具名必须回到 (string, true)，不能 panic。
    #[test]
    fn unknown_tool_is_error_result() {
        let (text, is_err) = execute_tool("not_a_tool", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }

    // 第 01 课：错误是 tool result。模型要读到「没有这个文件」，而不是进程崩掉。
    #[test]
    fn read_missing_file_is_error_result() {
        let (text, is_err) =
            execute_tool("read_file", r#"{"path":"/does/not/exist-byo-lesson01"}"#);
        assert!(is_err);
        assert!(!text.is_empty());
    }

    #[test]
    fn write_then_read_roundtrip() {
        let path = tmp_path("roundtrip.txt");
        let write_in = serde_json::json!({
            "path": path,
            "content": "haiku"
        })
        .to_string();
        let (wrote, write_err) = execute_tool("write_file", &write_in);
        assert!(!write_err, "{wrote}");

        let read_in = serde_json::json!({ "path": path }).to_string();
        let (body, read_err) = execute_tool("read_file", &read_in);
        let _ = fs::remove_file(&path);
        assert!(!read_err, "{body}");
        assert_eq!(body, "haiku");
    }

    #[test]
    fn bash_echo_returns_stdout() {
        let (text, is_err) = execute_tool("bash", r#"{"command":"echo lesson01"}"#);
        assert!(!is_err, "{text}");
        assert!(text.contains("lesson01"), "{text}");
    }
}
