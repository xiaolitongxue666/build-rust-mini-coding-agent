//! 第 11 课：把一个 Subagent 暴露成工具 `delegate_<name>`。
//!
//! 放在 crate 根，**不**进 `tools/`。课上的环是
//! `tool → subagent → agent → tool`。胶水留在集成层。
//! `Tool` 接口在 `tools`，谁都能 impl。

use std::sync::Arc;
use std::time::Instant;

use serde_json::Map;

use crate::api::ToolDef;
use crate::provider::Provider;
use crate::subagent::{self, Research, Subagent};
use crate::tools::{self, Registry, Tool};
use crate::ui::dimmed;

pub struct DelegateTool {
    pub subagent: Arc<dyn Subagent>,
}

impl Tool for DelegateTool {
    fn definition(&self) -> ToolDef {
        let mut input_schema = Map::new();
        input_schema.insert(
            "task".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Concrete description of what the subagent should do."
            }),
        );
        ToolDef {
            name: format!("delegate_{}", self.subagent.name()),
            description: self.subagent.description(),
            input_schema,
            required: vec!["task".to_string()],
        }
    }

    fn execute(&self, raw_input: &str) -> (String, bool) {
        let task = match tools::json_field(raw_input, "task") {
            Ok(t) => t,
            Err(err) => return (format!("invalid tool input: {err}"), true),
        };
        let name = self.subagent.name();
        println!("{}", dimmed(&format!("↳ delegating to {name} subagent")));
        let start = Instant::now();
        let result = self.subagent.run(&task);
        let elapsed = start.elapsed();
        println!(
            "{}",
            dimmed(&format!("← {name} subagent done ({elapsed:.1?})"))
        );
        match result {
            Ok(text) => (text, false),
            Err(err) => (format!("subagent error: {err}"), true),
        }
    }
}

/// 第 11 课：子 agent 要 Provider，不能 `init()`。两行一个：登记 + 挂 delegate。
pub fn register_subagents<P>(llm: &P, tools: &mut Registry) -> subagent::Registry
where
    P: Provider + Clone + Send + Sync + 'static,
{
    let mut subagents = subagent::Registry::new();
    subagents.register(Research {
        provider: llm.clone(),
        tools: crate::tools::default_registry().subset(&["read_file"]),
    });
    for sa in subagents.all() {
        tools.register(DelegateTool { subagent: sa });
    }
    subagents
}
