//! 第 11 课：只读调查子 agent。每次 `Run` 是新的 `Agent`，不带上次状态。
//! `read_file` 子集，没有 bash。`Quiet` + `LogPrefix` + `Confirm=nil`。
//! `MaxTurns = 10`。身份仍是 DeepSeek，带上当前模型名。

use crate::agent::Agent;
use crate::provider::Provider;
use crate::subagent::{begin, Subagent};
use crate::tools::Registry;

pub struct Research<P> {
    pub provider: P,
    pub tools: Registry,
}

fn research_system(model: &str) -> String {
    format!(
        "You are DeepSeek, a research subagent. The model serving this session is {model} \
         (DeepSeek, not Claude, not GPT).\n\n\
         Your job is to investigate the task you're given and return a concise, factual answer.\n\n\
         Rules:\n\
         - Use the tools available to look up information. Prefer fewer, more targeted reads over scanning everything.\n\
         - Return a short answer with the specific facts requested. No preamble.\n\
         - If the answer requires a path or identifier, include it verbatim.\n\
         - You have a limited number of tool calls; do not waste them."
    )
}

impl<P> Subagent for Research<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "research"
    }

    fn description(&self) -> String {
        "Investigate the codebase or filesystem and return a focused answer. \
         Prefer this over reading files yourself when the user asks ANY question \
         about the code — 'where is X', 'how does Y work', 'what does Z look like'. \
         The subagent has read_file access and its own context window, so it can \
         explore freely without polluting your conversation. Always pass a concrete \
         task description, not just the user's literal question."
            .to_string()
    }

    fn run(&self, task: &str) -> Result<String, String> {
        let _end = begin(self.name());
        let system = research_system(self.provider.model());
        // 第 15 课：clone 带上项目上下文。这里的 set_system 只换研究用的行为半边。
        let mut agent = Agent::new(self.provider.clone(), system, self.tools.clone());
        agent.name = self.name().to_string();
        agent.log_prefix = "  ↳ ".to_string();
        agent.quiet = true;
        agent.max_turns = 10;
        agent.send(task.to_string())
    }
}
