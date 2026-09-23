//! 第 01 课内层循环。第 02 课用 `use_gate` 在执行前插入审批，两个出口不变。
//! 第 03 课：循环只认 `Provider` + 通用 `Message`，不再直接打 Chat Completions。
//! 第 06 课：循环在 `messages` 上转，每次 `send` 重读整段。没有 session。
//! 第 07 课：每轮开头跑 `CompactionStrategy`。压缩在 `send` 外面，避免 Summarize 递归。
//! 第 10 课：循环在库里，不在 `main.rs`。
//! 第 11 课：收成 `Agent`。根和子 agent 各一份状态。`Quiet` / `LogPrefix` / `Confirm=None`
//! 把子 agent 和根分开。
//! 第 12 课：循环不再 import `ui`。转圈交给整屏状态行；`println!` 进管子。
//! 审批是 `Confirm` 回调（通道 + TUI），不是在 Update 里堵死。
//! 第 18 课：回调多一个 detail。只有本地 `write_file` 把统一 diff 放进去。

use std::sync::Arc;

use crate::api::{Block, Message, StopReason, ToolDef};
use crate::compact::{print_compaction, CompactionStrategy, NoCompaction};
use crate::gate::{confirm_decision, Decision, PromptRead};
use crate::provider::Provider;
use crate::tools::Registry;

/// 第 18 课：第二个参数是长文。`write_file` 放 diff，其它工具传空串。
pub type ConfirmFn = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// 第 11 课：一份对话的全部状态。根 REPL 一份，每次 `delegate_*` 再 new 一份。
pub struct Agent<P: Provider> {
    pub name: String,
    pub llm: P,
    pub tools: Registry,
    pub compact: Box<dyn CompactionStrategy + Send>,
    pub system: String,
    pub max_turns: usize,
    pub verbose: bool,
    /// 子 agent 用 `"  ↳ "`，根用空串。
    pub log_prefix: String,
    /// 子 agent 为 true：过程文本不回显，只交最终答案。
    pub quiet: bool,
    /// false = 课上 `Confirm: nil`，子 agent 自动过。
    pub use_gate: bool,
    /// 第 12 课：TUI 接线。`None` 时 `send_with` 走 `PromptRead`。
    pub confirm: Option<ConfirmFn>,
    /// 第 11 课：对外用 `Messages` / `SetMessages`。字段 pub 是为了 REPL 和命令拆借。
    pub messages: Vec<Message>,
}

impl<P: Provider> Agent<P> {
    pub fn new(mut llm: P, system: String, tools: Registry) -> Self {
        // 第 15 课：非空时只换行为半边。适配器把已经焊上的 AGENTS.md 再拼回去。
        if !system.is_empty() {
            llm.set_system(system.clone());
        }
        Self {
            name: String::new(),
            llm,
            tools,
            compact: Box::new(NoCompaction),
            system,
            max_turns: 20,
            verbose: false,
            log_prefix: String::new(),
            quiet: false,
            use_gate: false,
            confirm: None,
            messages: Vec::new(),
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn set_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
}

impl<P: Provider + Clone + Send + 'static> Agent<P> {
    /// 第 11 / 12 课：先 append user。TUI 走 `confirm` 回调；子 agent 自动过。
    pub fn send(&mut self, prompt: impl Into<String>) -> Result<String, String> {
        self.messages.push(Message::user_text(prompt.into()));
        let confirm = self.confirm.clone();
        let use_gate = self.use_gate;
        self.loop_body(|prompt, detail| match &confirm {
            Some(f) => {
                if f(prompt, detail) {
                    Decision::Yes
                } else {
                    Decision::No
                }
            }
            None if use_gate => Decision::No,
            None => Decision::Yes,
        })
    }

    /// 旧课 / 管道：审批仍走 `PromptRead`。
    pub fn send_with<R: PromptRead>(
        &mut self,
        prompt: impl Into<String>,
        input: &mut R,
    ) -> Result<String, String> {
        self.messages.push(Message::user_text(prompt.into()));
        self.loop_body(|prompt, detail| {
            if !detail.is_empty() {
                println!("{detail}");
            }
            confirm_decision(prompt, input)
        })
    }

    pub fn loop_turns_with<R: PromptRead>(
        &mut self,
        input: &mut R,
        compact: &dyn CompactionStrategy,
        verbose: bool,
    ) -> Result<String, String> {
        self.loop_body_with(compact, verbose, |prompt, detail| {
            if !detail.is_empty() {
                println!("{detail}");
            }
            confirm_decision(prompt, input)
        })
    }

    fn loop_body(&mut self, approve: impl FnMut(&str, &str) -> Decision) -> Result<String, String> {
        let compact = std::mem::replace(&mut self.compact, Box::new(NoCompaction));
        let verbose = self.verbose;
        let result = self.loop_body_with(compact.as_ref(), verbose, approve);
        self.compact = compact;
        result
    }

    fn loop_body_with(
        &mut self,
        compact: &dyn CompactionStrategy,
        verbose: bool,
        mut approve: impl FnMut(&str, &str) -> Decision,
    ) -> Result<String, String> {
        let mut origin = self.messages.len();
        let mut final_text = String::new();
        for _turn in 0..self.max_turns {
            let before_len = self.messages.len();
            // 第 17 课：只替换 messages。system 留在 provider 上，不跟这次压缩一起改。
            match compact.compact(&self.messages, &self.llm) {
                Ok(next) => {
                    if verbose && next.len() != self.messages.len() {
                        print_compaction(&self.messages, &next);
                    }
                    let dropped = before_len.saturating_sub(next.len());
                    origin = origin.saturating_sub(dropped);
                    self.messages = next;
                }
                Err(err) => {
                    println!(
                        "{}compaction error: {err} (continuing without)",
                        self.log_prefix
                    );
                }
            }

            // 第 12 课：不再 `spin_until`。状态行转圈；这里只堵线程。
            let defs = self.tools.definitions();
            let resp = match self.llm.send(&self.messages, &defs) {
                Ok(r) => r,
                Err(err) => {
                    println!("api error: {err}");
                    return Ok(final_text.trim().to_string());
                }
            };

            for block in &resp.content {
                if block.ty == crate::api::BlockType::Text && !block.text.is_empty() {
                    if !self.quiet {
                        println!("{}", block.text);
                    }
                    if !final_text.is_empty() {
                        final_text.push('\n');
                    }
                    final_text.push_str(&block.text);
                }
            }

            // 第 01 课陷阱 / 第 06 课：必须把 assistant 原样 append。漏了，下一轮孤立的 tool_result 会 400。
            let tool_uses: Vec<Block> = resp
                .content
                .iter()
                .filter(|b| b.ty == crate::api::BlockType::ToolUse)
                .cloned()
                .collect();
            self.messages.push(Message::assistant(resp.content));

            if resp.stop_reason != StopReason::ToolUse || tool_uses.is_empty() {
                return Ok(final_text.trim().to_string());
            }

            let mut results = Vec::new();
            for call in tool_uses {
                match self.execute_tool(&call.tool_name, &call.tool_input, &mut approve) {
                    ToolOutcome::Aborted => {
                        self.messages.truncate(origin.saturating_sub(1));
                        return Ok(final_text.trim().to_string());
                    }
                    ToolOutcome::Done(result, is_err) => {
                        if is_err {
                            eprintln!("[tool error] {}", truncate(&result, 200));
                        }
                        results.push(Block::tool_result(call.tool_use_id, result, is_err));
                    }
                }
            }
            // 第 06 课：一条 user 消息装着本轮全部 tool_result；`tool_use_id` 必须对上模型给的 id。
            self.messages.push(Message::tool_results(results));
        }
        Err(format!("max turns ({}) reached", self.max_turns))
    }

    /// 第 11 课：打 `{LogPrefix}[tool]`。`use_gate=false` 就是课上 Confirm=nil。
    fn execute_tool(
        &self,
        name: &str,
        raw_input: &str,
        approve: &mut impl FnMut(&str, &str) -> Decision,
    ) -> ToolOutcome {
        println!("{}[tool] {name} {raw_input}", self.log_prefix);
        if self.use_gate {
            // 第 18 课：只在落盘前问。detail 不写进 tool_result，模型看不到 diff。
            let (prompt, detail) = crate::write_diff::write_approval(name, raw_input);
            match approve(&prompt, &detail) {
                Decision::Abort => return ToolOutcome::Aborted,
                Decision::No => {
                    return ToolOutcome::Done("user denied this tool call".to_string(), true);
                }
                Decision::Yes => {}
            }
        }
        // 第 14 课：本地工具和 MCP 工具走同一条 approve。门不看调用是不是子进程。
        let (result, is_err) = self.tools.execute(name, raw_input);
        ToolOutcome::Done(result, is_err)
    }
}

enum ToolOutcome {
    Done(String, bool),
    Aborted,
}

/// `use_gate`：总体和第 02 课为 true；第 01 课 demo 为 false。
/// 旧课 demo / 单测走默认 `NoCompaction`。
pub fn agent_loop<P, R>(
    llm: &mut P,
    tools: &[ToolDef],
    messages: Vec<Message>,
    input: &mut R,
    use_gate: bool,
) -> Vec<Message>
where
    P: Provider + Clone + Send + 'static,
    R: PromptRead,
{
    agent_loop_with(llm, tools, messages, input, use_gate, &NoCompaction, false)
}

pub fn agent_loop_with<P, R>(
    llm: &mut P,
    tools: &[ToolDef],
    messages: Vec<Message>,
    input: &mut R,
    use_gate: bool,
    compact: &dyn CompactionStrategy,
    verbose: bool,
) -> Vec<Message>
where
    P: Provider + Clone + Send + 'static,
    R: PromptRead,
{
    let registry = if tools.is_empty() {
        Registry::new()
    } else {
        crate::tools::default_registry().clone()
    };
    let mut agent = Agent::new(llm.clone(), String::new(), registry);
    agent.use_gate = use_gate;
    agent.set_messages(messages);
    let _ = agent.loop_turns_with(input, compact, verbose);
    agent.messages
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
