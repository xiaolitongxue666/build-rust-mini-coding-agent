# 15 · 项目上下文 AGENTS.md

对照：[Project context with AGENTS.md](https://www.byoharness.dev/chapters/15-agents-md.html) · 本课代码：[`examples/15-agents-md.rs`](../../examples/15-agents-md.rs)

系统提示写 harness 怎么干活。`AGENTS.md` 写**这个仓库**是什么样。两段拼成一次请求里的 system，不当成用户消息。`/clear` 清的是对话，这段还在。

## 读一次

`repl::run_repl` 启动时读 cwd 下的 `AGENTS.md`，和 `mcp.json` 一样相对工作目录。

| 情况 | 结果 |
|---|---|
| 文件在 | 前面加 `# Project context (from AGENTS.md)`，接到行为提示后面 |
| 文件不在 | 空串，不打日志 |
| 读失败 | 同样当没有 |

不往父目录找。不热重载。改完文件要重启 harness。课上的 `/context`、`/reload-context` 是练习，这里不写。

## 两半不要贴两遍

`DeepSeekProvider` 分开记：

- `behavior`：`system_prompt`，或子 agent 换上的研究提示
- `project_context`：文件内容
- `system`：拼好的那一段，请求里插的是它

`/model` 只重写 `behavior`，上下文留着。Research 的 `set_system` 也一样：clone 已经带上上下文，换的是研究用的行为半边。

`Agent::new` 传入的是行为半边。适配器在 `set_system` 里再把上下文拼回去。如果传入的已经是拼好的全文，就会贴两遍。

## 跑起来

```bash
bash scripts/run.sh            # 总体，读 cwd 的 AGENTS.md
bash scripts/run.sh 15
bash scripts/test-lessons.sh 15
```

不要把密钥、本机路径写进 `AGENTS.md`。文件会进 git，也会进每次请求。
