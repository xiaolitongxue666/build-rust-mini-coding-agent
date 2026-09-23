//! 第 14 课：[Adding MCP support](https://www.byoharness.dev/chapters/14-mcp-support.html)
//!
//! 课上的客户端是 Go SDK 外面薄薄一层。这里对应官方 Rust SDK [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk)。
//! JSON-RPC、握手、stdio / Streamable HTTP 都在 SDK 里。
//!
//! harness 的循环是同步的（第 03 课 blocking HTTP，第 12 课后台线程）。
//! SDK 是 async，所以只在这一层 `block_on`，不把 tokio 漏进 `Tool` / `agent`。
//!
//! 第 09 课 `Tool::execute` 没有 `Context`。飞行中的调用跟会话走：
//! `close` 取消 `RunningService`，stdio 子进程一起结束。
//! 不要在 `execute` 里再 `spawn` 一个脱离这次会话的任务。

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use http::{HeaderName, HeaderValue};
use rmcp::model::{CallToolRequestParams, ClientConfig, ContentBlock, Implementation};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::transport::{ConfigureCommandExt, IntoTransport, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Map, Value};

type Session = RunningService<RoleClient, ClientConfig>;

const CLIENT_NAME: &str = "bettatech-harness";
const CLIENT_VERSION: &str = "0.1";

pub struct Client {
    inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    name: String,
    session: Mutex<Option<Session>>,
}

impl Client {
    pub(crate) fn connect_stdio(
        name: &str,
        command: &str,
        args: &[String],
    ) -> Result<Self, String> {
        // tokio 在 Windows 上不查 PATHEXT，裸 `npx` 找不到 `npx.cmd`。
        // 自己按 PATH 解一次；解不到就原样交给 spawn，错误仍由 dial 跳过。
        let command = match resolve_program(command) {
            Some(path) => tokio::process::Command::new(path),
            None => tokio::process::Command::new(command),
        };
        let args = args.to_vec();
        let name = name.to_string();
        // spawn 必须在 runtime 里。tokio 的 Command 在外面会报 no reactor。
        block_on(async move {
            let transport = TokioChildProcess::new(command.configure(|cmd| {
                cmd.args(&args);
            }))
            .map_err(|err| format!("mcp connect {name}: {err}"))?;
            open_session(name, transport).await
        })
    }

    pub(crate) fn connect_http(
        name: &str,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Self, String> {
        let mut custom = HashMap::new();
        for (key, value) in headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|err| format!("http server {name:?} bad header {key:?}: {err}"))?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|err| format!("http server {name:?} bad header {key:?}: {err}"))?;
            // 课上的 Authorization 是整段字面量（含 Bearer）。
            // SDK 的 auth_header() 会再加一层 Bearer，所以全部走 custom headers。
            custom.insert(header_name, header_value);
        }
        let url = url.to_string();
        let name = name.to_string();
        block_on(async move {
            let transport = StreamableHttpClientTransport::from_config(
                StreamableHttpClientTransportConfig::with_uri(url).custom_headers(custom),
            );
            open_session(name, transport).await
        })
    }

    pub(crate) fn shared(&self) -> Arc<ClientInner> {
        Arc::clone(&self.inner)
    }

    /// 第 14 课：进程退出前关掉。stdio 杀子进程，HTTP 拆掉长连接。
    pub fn close(&self) {
        self.inner.close();
    }

    #[allow(clippy::await_holding_lock)]
    pub(crate) fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, String> {
        let inner = Arc::clone(&self.inner);
        block_on(async move {
            let guard = inner.lock_session()?;
            let Some(session) = guard.as_ref() else {
                return Err(format!("mcp session closed: {}", inner.name));
            };
            session
                .list_all_tools()
                .await
                .map_err(|err| err.to_string())
        })
    }
}

impl ClientInner {
    fn lock_session(&self) -> Result<std::sync::MutexGuard<'_, Option<Session>>, String> {
        self.session
            .lock()
            .map_err(|_| format!("mcp session poisoned: {}", self.name))
    }

    fn close(&self) {
        let session = match self.session.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(session) = session {
            // cancel 会等传输收尾。丢了错误也要继续关下一个 server。
            let _ = block_on(session.cancel());
        }
    }

    pub(crate) fn call_tool(
        &self,
        remote_name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<(String, bool), String> {
        let name = remote_name.to_string();
        // 锁只保证同一个 server 的 RPC 不交错。课上接受串行。
        #[allow(clippy::await_holding_lock)]
        block_on(async {
            let guard = self.lock_session()?;
            let Some(session) = guard.as_ref() else {
                return Err(format!("mcp session closed: {}", self.name));
            };
            let mut params = CallToolRequestParams::new(name);
            if let Some(arguments) = arguments {
                params = params.with_arguments(arguments);
            }
            let result = session
                .call_tool(params)
                .await
                .map_err(|err| err.to_string())?;
            Ok((
                join_content(&result.content),
                result.is_error.unwrap_or(false),
            ))
        })
    }
}

async fn open_session<T, E, A>(name: String, transport: T) -> Result<Client, String>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let session = harness_client()
        .serve(transport)
        .await
        .map_err(|err| format!("mcp connect {name}: {err}"))?;
    Ok(Client {
        inner: Arc::new(ClientInner {
            name,
            session: Mutex::new(Some(session)),
        }),
    })
}

fn harness_client() -> ClientConfig {
    let mut config = ClientConfig::default();
    config.client_info = Implementation::new(CLIENT_NAME, CLIENT_VERSION);
    config
}

/// 第 14 课：多个 content 块拼成循环认的那一个字符串。
/// 非文本留占位，不静默丢掉。
fn join_content(blocks: &[ContentBlock]) -> String {
    let mut parts = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            ContentBlock::Text(text) => parts.push(text.text.clone()),
            other => parts.push(format!("[non-text content block: {}]", content_kind(other))),
        }
    }
    parts.join("\n")
}

fn content_kind(block: &ContentBlock) -> &'static str {
    match block {
        ContentBlock::Text(_) => "text",
        ContentBlock::Image(_) => "image",
        ContentBlock::Audio(_) => "audio",
        ContentBlock::Resource(_) => "resource",
        ContentBlock::ResourceLink(_) => "resource_link",
        _ => "unknown",
    }
}

/// Windows 的 `npx` / `uvx` 实际是 `.cmd`。tokio 不查 `PATHEXT`，这里先解开。
fn resolve_program(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() || path.components().count() > 1 {
        return path.is_file().then(|| path.to_path_buf());
    }
    let path_var = std::env::var_os("PATH")?;
    let mut suffixes = Vec::new();
    if cfg!(windows) {
        let pathext =
            std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
        suffixes.push(String::new());
        for ext in pathext.split(';') {
            let ext = ext.trim();
            if !ext.is_empty() {
                suffixes.push(ext.to_string());
            }
        }
    } else {
        suffixes.push(String::new());
    }
    for dir in std::env::split_paths(&path_var) {
        for suffix in &suffixes {
            let candidate = dir.join(format!("{program}{suffix}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("mcp tokio runtime")
    })
}

/// 多线程 runtime 上并行 `block_on` 会互相踩。一条锁把 MCP 调用收成串行。
fn block_on<F: Future>(future: F) -> F::Output {
    static GATE: Mutex<()> = Mutex::new(());
    let _guard = GATE.lock().unwrap_or_else(|poison| poison.into_inner());
    runtime().block_on(future)
}
