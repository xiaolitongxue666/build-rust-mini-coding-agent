# 14 · 接上 MCP

对照：[Adding MCP support](https://www.byoharness.dev/chapters/14-mcp-support.html) · 本课代码：[`examples/14-mcp-support.rs`](../../examples/14-mcp-support.rs)

第 09 课的 Registry 收的是本地工具：丢一个文件，`definition` + `execute`。MCP 是**进程外面**的工具。Git、文件系统、别人写好的 server，都走同一条协议。

本课只接 `tools/list` 和 `tools/call`。resources / prompts 先不接。

## 协议不规定你要下载什么

MCP 是跟传输无关的 JSON-RPC。

| 传输 | 本仓库 |
|---|---|
| stdio | 启动时拉起子进程，stdin/stdout 说话。会话结束杀掉。 |
| http | Streamable HTTP。URL 在远端，这边不装包。 |
| WebSocket | 课上标了少见。配置里不认。 |

stdio 的命令可以是早就装好的二进制，也可以是 `uvx` / `npx -y` 这种第一次才去拉的启动器。下不下载是那条命令自己的事。样例在 [`mcp.example.json`](../../mcp.example.json)。真正读的是 cwd 下的 `mcp.json`（gitignore，里面可能有 token）。

`${VAR}` 和 `$VAR` 在 command、args、url、header 里展开。密钥留在环境里。

## 桥

```text
Registry
  bash                      本地
  read_file                 本地
  git_status                MCPTool → git server
  filesystem_read_file      MCPTool → filesystem server
```

循环、审批、`/tools` 都不知道谁是远程的。模型叫 `git_status`，门问一句 approve，然后 `MCPTool::execute` 去做 RPC。

课上用 Go SDK。这里用官方 Rust SDK [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk)。不手写 JSON-RPC，也不绑到 DeepSeek 的客户端上。SDK 是 async，只在 `src/mcp/client.rs` 里 `block_on`。第 03 课的循环继续是同步的。

第 09 课的 `execute` 没有 `Context`。取消跟会话走：退出时 `close`，stdio 子进程一起结束。不要在 `execute` 里再拉一个脱离会话的任务。

## 什么时候读配置

`repl::run_repl` 是本仓库的接线层（课上这段在 `main.go`）。`setup` 放在两件事**前面**：

1. `register_subagents`。子 agent 的 `Subset` 这时才能挑到远程工具。Research 仍然只拿 `read_file`，那是策展，不是漏登记。
2. `run_tui`。连接失败打在真终端上，不进第 12 课的 stdout 管子。

路径相对 cwd，不相对二进制。没有热重载。`tools/list` 每个 server 只打一次，而且是串行的。课上说生产可以并发；学习项目接受启动时多等几秒。

文件不存在：不打日志。某一条 server 起不来、名单拉失败、某个工具的 schema 认不出：打一行，跳过，接着跑。

## 名字要加前缀

Registry 是扁平的。两个 server 都叫 `read_file`，后登记的会盖住先登记的，本地的 `read_file` 也会被盖住。对外叫 `git_status`，打 RPC 时仍用 server 上的原名。

schema 里出现 `$ref` 就跳过**这一个工具**，不拆掉整个 server。DeepSeek 的 function parameters 不吃这种引用。

## 审批

远程工具没审计过。看起来只读也不自动批准。门还是第 02 课那一处。

## 跑起来

```bash
cp mcp.example.json mcp.json   # 按本机改命令；mcp.json 不进 git
bash scripts/run.sh            # 总体
bash scripts/run.sh 14
bash scripts/test-lessons.sh 14
```

`/tools` 里本地三个名字还在。配了 server 的话，后面跟着 `git_…` 这种前缀名。

默认测试不拉 uvx。stdio 往返用 `src/bin/mcp-stdio-fixture.rs`，只在 `cargo test` 里当夹具。
