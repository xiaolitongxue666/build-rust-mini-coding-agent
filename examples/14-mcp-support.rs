//! 第 14 课：[Adding MCP support](https://www.byoharness.dev/chapters/14-mcp-support.html)
//!
//! 本课小 demo：外部 MCP server 接进同一套 Registry。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! Tool::execute  ←  MCPTool  ←  rmcp Client  ←  stdio 子进程 / HTTP
//! ```
//!
//! 循环分不清本地工具和远程工具。审批仍在 harness 层，不因为「看起来只读」就自动过。
//!
//! 配置是 cwd 下的 `mcp.json`（gitignore）。样例是 `mcp.example.json`。
//! 文件不存在就当没装 MCP，不打日志。某个 server 起不来就跳过，agent 继续跑。
//!
//! 工具名加 server 前缀：`git_status`、`filesystem_read_file`。
//! RPC 仍用 server 上的原名。
//!
//! 启动：`bash scripts/run.sh 14`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use build_rust_mini_coding_agent::mcp;

    #[test]
    fn lesson_14_missing_file_is_opt_in() {
        let path = std::env::temp_dir().join("byo-lesson14-missing-mcp.json");
        let _ = fs::remove_file(&path);
        assert!(mcp::load_config(&path).unwrap().is_none());
    }

    #[test]
    fn lesson_14_expand_env_keeps_secrets_out_of_the_file() {
        let text = mcp::expand_env("prefix-$$${UNSET_BYO_LESSON14_TOKEN}-tail");
        assert_eq!(text, "prefix-$-tail");
    }

    #[test]
    fn lesson_14_names_are_prefixed() {
        assert_eq!(mcp::exposed_name("git", "status"), "git_status");
        assert_eq!(
            mcp::exposed_name("filesystem", "read_file"),
            "filesystem_read_file"
        );
    }
}
