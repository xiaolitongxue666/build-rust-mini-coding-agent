//! 第 14 课：[Adding MCP support](https://www.byoharness.dev/chapters/14-mcp-support.html)
//!
//! 外部 MCP server 和本地 `Tool` 之间的桥。循环只看见 Registry 里的一张扁平名单。
//!
//! ```text
//! bash / read_file / write_file     本地
//! git_status                        MCPTool → git server
//! filesystem_read_file              MCPTool → filesystem server
//! ```
//!
//! 登记发生在接线层（`repl::run_repl`），而且必须在两件事之前：
//! 1. `register_subagents`——子 agent 的 `Subset` 才能挑到远程工具。
//! 2. `run_tui`——连接错误打在真终端上，不进第 12 课的 stdout 管子。
//!
//! 不写 `/reload-mcp`。不把 MCP 工具自动批准。

mod client;
mod register;

pub use client::Client;
pub use register::{close_all, expand_env, exposed_name, load_config, setup, setup_at, Config};
