//! 第 09 课：[Plug-and-play tools](https://www.byoharness.dev/chapters/09-plug-and-play-tools.html)
//!
//! 工具是**可叠加**的（能同时有很多），不是互斥的（Provider / 压缩各只有一个）。
//! 所以不是 `main` 换一行，而是 Registry。
//!
//! Rust 没有 Go 那种「同包每个文件跑 `init()`」。同一模块里
//! `default_registry()` 登记；加工具 = 新文件 + 这里一行，**不动** `main.rs`。
//! 不要拆成 `tools/bash/` 子包，否则又要在某处列一遍。
//! 第 10 课：这就是课上 `internal/tool/`——接口和每个工具同目录，不套 `src/internal/`。
//!
//! `Definitions` 必须按名字排序：HashMap 迭代顺序随机，打乱字节会毁掉以后的 prompt cache。
//! 第 11 课：`Subset` 给子 agent 一份只读工具面。这是策展，不是沙箱。
//! 不写 `web_fetch`（课上练习）。`DelegateTool` 不进本模块，以免 `tools → subagent → agent → tools`。

mod bash;
mod read_file;
mod write_file;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::{json, Map, Value};

use crate::api::ToolDef;

pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDef;
    fn execute(&self, input: &str) -> (String, bool);
}

pub struct Registry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.register_arc(Arc::new(tool));
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name;
        self.tools.insert(name, tool);
    }

    /// 第 11 课：`Subset` 从已有登记里挑名字，组一份新 Registry。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn subset(&self, names: &[&str]) -> Registry {
        let mut out = Registry::new();
        for name in names {
            if let Some(tool) = self.get(name) {
                out.register_arc(tool);
            }
        }
        out
    }

    /// 第 09 课：先按名字排序，再给模型。顺序不稳会打乱 prompt cache。
    pub fn definitions(&self) -> Vec<ToolDef> {
        let mut names: Vec<&String> = self.tools.keys().collect();
        names.sort();
        names
            .into_iter()
            .map(|name| self.tools[name].definition())
            .collect()
    }

    /// 第 01 / 09 课：未知名字走 error result，不要 panic。
    pub fn execute(&self, name: &str, input: &str) -> (String, bool) {
        match self.tools.get(name) {
            Some(tool) => tool.execute(input),
            None => (format!("unknown tool: {name}"), true),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Registry {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}

fn build_default() -> Registry {
    let mut registry = Registry::new();
    // 第 09 课：加工具在这里加一行，不要去改 main。
    registry.register(bash::BashTool);
    registry.register(read_file::ReadFileTool);
    registry.register(write_file::WriteFileTool);
    registry
}

/// 课上的包级 `Default`。需要配置的工具不能自动登记（第 11 课）。
pub fn default_registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(build_default)
}

pub fn default_tool_defs() -> Vec<ToolDef> {
    default_registry().definitions()
}

pub fn dispatch_tool(name: &str, raw_input: &str) -> (String, bool) {
    default_registry().execute(name, raw_input)
}

pub(crate) fn string_prop(fields: &[(&str, &str)]) -> Map<String, Value> {
    let mut props = Map::new();
    for (name, description) in fields {
        props.insert(
            (*name).to_string(),
            json!({"type": "string", "description": description}),
        );
    }
    props
}

pub fn json_field(raw: &str, key: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTool;

    impl Tool for FakeTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "aaa_fake".to_string(),
                description: "test".to_string(),
                input_schema: Map::new(),
                required: vec![],
            }
        }

        fn execute(&self, _input: &str) -> (String, bool) {
            ("ok".to_string(), false)
        }
    }

    #[test]
    fn definitions_are_sorted_by_name() {
        let defs = default_registry().definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert_eq!(names, vec!["bash", "read_file", "write_file"]);
    }

    #[test]
    fn register_is_additive() {
        let mut registry = Registry::new();
        registry.register(bash::BashTool);
        registry.register(FakeTool);
        let names: Vec<String> = registry.definitions().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["aaa_fake".to_string(), "bash".to_string()]);
    }

    #[test]
    fn unknown_tool_is_error_result() {
        let (text, is_err) = Registry::new().execute("missing", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }

    #[test]
    fn subset_is_curation_not_sandbox() {
        let subset = default_registry().subset(&["read_file"]);
        let names: Vec<String> = subset.definitions().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["read_file".to_string()]);
        let (text, is_err) = subset.execute("bash", r#"{"command":"pwd"}"#);
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }
}
