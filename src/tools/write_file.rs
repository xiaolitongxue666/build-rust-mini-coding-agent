//! 第 09 课：一个文件一个工具。登记在 `default_registry()`，不进 main。

use std::fs;

use crate::api::ToolDef;
use crate::tools::{json_field, string_prop, Tool};

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "write_file".to_string(),
            description: "Write content to a file (creating or overwriting it).".to_string(),
            input_schema: string_prop(&[
                ("path", "Filesystem path to write."),
                ("content", "The bytes to write."),
            ]),
            required: vec!["path".to_string(), "content".to_string()],
        }
    }

    fn execute(&self, raw_input: &str) -> (String, bool) {
        let path = match json_field(raw_input, "path") {
            Ok(v) => v,
            Err(e) => return (e, true),
        };
        let content = match json_field(raw_input, "content") {
            Ok(v) => v,
            Err(e) => return (e, true),
        };
        match fs::write(&path, content) {
            Ok(()) => (format!("wrote {path}"), false),
            Err(e) => (e.to_string(), true),
        }
    }
}
