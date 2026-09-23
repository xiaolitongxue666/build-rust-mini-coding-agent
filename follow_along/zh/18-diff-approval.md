# 18 · 写文件前看 diff

对照：[Diff approval for writes](https://www.byoharness.dev/chapters/18-diff-approval.html) · 本课代码：[`examples/18-diff-approval.rs`](../../examples/18-diff-approval.rs)

`approve?` 对 `bash ls` 够用。对 `write_file` 不够：你批准的是「它想写点什么」，内容要等写完才能看见。

模型发出的是 path 和全文。磁盘还没动。harness 读当前文件，算出统一 diff，再问一次 y/n。整份 diff 一次决定，不按块批准，也不打开编辑器。

## 三种结果

| 磁盘上 | 弹层 |
|---|---|
| 没有这个文件 | `--- /dev/null`，正文全是 `+` |
| 有文件，内容不同 | `path (current)` / `path (proposed)`，上下文 3 行 |
| 字节相同 | `(no changes)` |

参数不是 `path` + `content` 时，退回普通 `approve?`，不编一份空 diff。

## 谁看得到

diff 只给审批的人。模型拿回的仍是 `wrote …` 或 `user denied this tool call`。要解释为什么拒绝，下一句自己说。

只认本地工具名 `write_file`。MCP 的 `filesystem_write_file` 不知道是不是「整文件覆盖」，继续普通提示。

管道模式把 diff 打在 `y/n` 前面。整屏里 diff 占视口，黄框还是一次 y/n，`↑↓` 翻页。课上用 Chroma 的 diff 词法；这里继续 crossterm：`+` 绿、`-` 红、`@@` 暗。

diff 是按下键那一刻读到的文件。别的终端同时改了同一份，批准后写下去的仍是模型给出的全文。按文本行比较，不认二进制。

## 跑起来

```bash
bash scripts/run.sh
bash scripts/run.sh 18
bash scripts/test-lessons.sh 18
```

默认测试不打真实模型。实机里让它新建一个小文件，弹层应是全绿的新增；再改同一份，才能看到 `-` 和 `+`。按 `n` 之后模型只知道这次被拒绝。
