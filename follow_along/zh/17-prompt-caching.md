# 17 · 提示词缓存

对照：[Prompt caching](https://www.byoharness.dev/chapters/17-prompt-caching.html) · 本课代码：[`examples/17-prompt-caching.rs`](../../examples/17-prompt-caching.rs)

每一轮都把 system、工具名单、整段历史再发一次。不变的那一段，服务器可以存成前缀。后面同一段只算命中，按命中价收。

## 课上的断点，这里不发

课上给 Claude 的 system 加 `cache_control`，把断点放在 system 末尾。工具名单跟着 system 一起进缓存，历史放在断点外面。压缩改历史，也弄不坏那一块。

DeepSeek 没有这个字段，磁盘缓存默认开着：[Context Caching](https://api-docs.deepseek.com/guides/kv_cache)。请求里不要加 `cache_control`。命中的是**完整的前缀单元**，不是「断点以前随便多长都算」。

所以这边稳住三件事：

1. system 永远是第一条消息，里面不放时间、随机数。`AGENTS.md` 一改，这条前缀就变了。
2. 工具按名字排序再编码。同一套工具必须是同一串 JSON。
3. 压缩只改 `messages`。system 留在 provider 上。

历史变长、而且旧消息原样留在前面时，下一轮可以对上上一轮的单元。`/compact` 砍掉或改写开头之后，旧单元对不上，下一轮要重新算那段。system 本身的字节没变，等公共前缀被认出来以后还能单独命中。默认策略是 `NoCompaction`，不到阈值不动刀。

不要加第二、第三个断点。DeepSeek 这边也没有地方加。

## `/tokens` 已经接着

第 16 课的账本认 `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens`。两栏都是 0 时，再读 OpenAI 兼容的 `prompt_tokens_details.cached_tokens`，折成同样的命中 / 未命中。有 DeepSeek 两栏时以那两栏为准，不重复加。

DeepSeek 不另收 Claude 那种写入加价（贵 25% 的 cache write）。未命中按输入价，命中按命中价。`cache write` 那一行保持不打印。命中价已经在第 16 课的人民币表里。

缓存是尽力而为，不保证每次命中。不用了会清掉，通常是几小时到几天，不是课上 Claude 的大约 5 分钟。

## 跑起来

```bash
bash scripts/run.sh
bash scripts/run.sh 17
bash scripts/test-lessons.sh 17
```

默认测试不打真实模型。实机里多聊几轮再 `/tokens`，命中数会出现在 cache hit 那一行。改 `AGENTS.md` 或中途 `/compact summarize` 之后，下一轮命中会掉下去。
