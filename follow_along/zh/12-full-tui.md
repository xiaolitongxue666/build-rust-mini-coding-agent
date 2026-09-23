# 12 · 整屏 TUI

对照：[The full TUI](https://www.byoharness.dev/chapters/12-full-tui.html) · 本课代码：[`examples/12-full-tui.rs`](../../examples/12-full-tui.rs)

第 08 课的框是一次性的：收一行就退，循环继续往 stdout 印。本课换老板——**一个程序占整屏**。

课上是 Bubble Tea。本仓库继续第 08 课的 crossterm MVU，不上 ratatui。管子、状态机、审批通道、循环不认 UI，和原章同一套。

## 时间线反了

以前：提交 → 循环卡住 → 印完 → 再问你。

现在：提交 → 循环丢到后台线程 → 界面继续收「又来一行 / 要审批 / 跑完了」。PgUp 还能翻。

三个状态：

| 从 | 事件 | 到 |
|---|---|---|
| 空闲 | 回车 | 在跑 |
| 在跑 | `Confirm` | 等审批 |
| 等审批 | y / n | 在跑 |
| 在跑 | 循环返回 | 空闲 |

## 管子

循环、斜杠、工具还是 `println!`。把 stdout 接到管子，搬运工一行行贴进视口。界面画到**复制出来的原 stdout**（备用屏），否则自己的画也进管子，死循环。

第 04 课那个 `\r` 转圈必须停。它写进管子会把屏画花。本课转圈在输入框上面：`⠹ thinking... · research`。

## 审批走通道

后台线程不能在界面线程里死等按键。发一张「请审批」纸条，自己堵在通道门口；你按 y/n，往通道里写，线程才继续。循环只调用 `Confirm`，不知道后面是 TUI。

`apply` / Update 里不准 `println!`。字会绕回自己，最后堵死。

## 环

`ui` 读 `subagent::Active()`。若 `agent` 再 import `ui`，就是 `agent → ui → subagent → agent`。

拆法：循环不再认 UI。转圈交给状态行；压缩对比本来就是 `println!`。循环只吐字，不知道有没有整屏。

`PromptRead` 搬到 `gate`，旧课管道审批还在。

## 视口

在底部才自动跟着新行。PgUp 停跟；End 再跟。底栏永远 5 行，状态切换不抖。

备用屏：Ctrl+D 回到原来的终端。

## 本课不做

不上 debug 面板、MCP 进度、banner 闪光、token 状态栏（那些是更后的课叠在 HEAD 上的）。没有「取消当前回合」——课上 Ctrl+D 退整进程。管道 / `BYO_PLAIN_INPUT=1` 仍是 `stdin.lines()`。

## 跑起来

```bash
bash scripts/run.sh 12
```

跑起来时 PgUp，跟应停；End 再跟。委托出去，状态行应出现 `research`。工具审批是黄框，按一下 y/n。
