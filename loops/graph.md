# Graph

后学：[第十四讲 从单循环到图工程](https://walkinglabs.github.io/learn-harness-engineering/zh/lectures/lecture-14-graph-engineering/)

loop 是 implement 节点内部的小循环。图只决定节点之间怎么走。不引入 LangGraph。

`pause_before: merge`

## 共享状态

| 字段 | 谁写 | 并行写入时 |
|---|---|---|
| requirements | 武装时写入 | 覆盖 |
| code | implement | 覆盖 |
| review | verify（pass / fail） | 覆盖 |
| attempts | verify 失败时 +1 | 求和 |
| current | 路由 | 覆盖 |
| thread | 武装时写入 | 覆盖 |

运行副本和 checkpoint 在 `.local/graph/<thread>/state.md`。

## 节点

| 节点 | 类型 | 内部 | 写入 |
|---|---|---|---|
| implement | agent | 上一讲的 maker（`BYO_ONCE`） | code |
| verify | 确定性 | `goal.md` 验证命令退出码；新上下文，不继承 implement 对话 | review |
| pause | 人 | `/graph approve` | current → merge |
| merge | 确定性 | 只写完成状态，不自动 `git commit` | current → done |

## 边

implement → verify →（路由）pause 或 implement → merge → 结束

## 路由

| 当前 | 条件 | 下一节点 |
|---|---|---|
| implement | 做完一轮 | verify |
| verify | review == pass | pause |
| verify | review == fail 且未到最大回合 | implement |
| verify | review == fail 且到顶 | blocked |
| pause | `/graph approve` | merge |
| merge | 收尾写完 | done |
