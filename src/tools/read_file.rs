//! 第 09 课：一个文件一个工具。登记在 `default_registry()`，不进 main。

use std::fs;

use crate::api::ToolDef;
use crate::tools::{json_field, string_prop, Tool};

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read_file".to_string(),
            description: "Read the contents of a file at the given path.".to_string(),
            input_schema: string_prop(&[("path", "Filesystem path to read.")]),
            required: vec!["path".to_string()],
        }
    }

    fn execute(&self, raw_input: &str) -> (String, bool) {
        let path = match json_field(raw_input, "path") {
            Ok(v) => v,
            Err(e) => return (e, true),
        };
        match fs::read_to_string(&path) {
            Ok(data) => (data, false),
            Err(e) => (e.to_string(), true),
        }
    }
}
