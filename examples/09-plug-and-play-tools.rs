//! 第 09 课：[Plug-and-play tools](https://www.byoharness.dev/chapters/09-plug-and-play-tools.html)
//!
//! 本课小 demo：工具从「切片 + switch」收成 Registry。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! interface → 可叠加的 impl → Registry
//! ```
//!
//! 循环只认 `registry.definitions()` 和 `registry.execute()`，不知道有哪些工具。
//!
//! | 关切 | 谁管 |
//! |---|---|
//! | 打 `[tool]` | gate / main |
//! | 审批 | gate（confirm） |
//! | 分发 | `Registry::execute` |
//!
//! Rust 没有 Go `init()`。同一模块 `default_registry()` 登记。
//! 加工具：`src/tools/` 新文件 + `mod.rs` 一行，不动 `main.rs`。
//!
//! `Definitions` 按名字排序。同名后登记的覆盖先登记的。
//! 本课不写 `Subset`、不写 `web_fetch`。
//!
//! 启动：`bash scripts/run.sh 09`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::api::ToolDef;
    use build_rust_mini_coding_agent::tools::{default_registry, Registry, Tool};
    use serde_json::Map;

    struct ExtraTool;

    impl Tool for ExtraTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "zzz_extra".to_string(),
                description: "additive".to_string(),
                input_schema: Map::new(),
                required: vec![],
            }
        }

        fn execute(&self, _input: &str) -> (String, bool) {
            ("extra".to_string(), false)
        }
    }

    #[test]
    fn lesson_09_default_names_are_sorted() {
        let names: Vec<String> = default_registry()
            .definitions()
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "bash".to_string(),
                "read_file".to_string(),
                "write_file".to_string()
            ]
        );
    }

    #[test]
    fn lesson_09_register_adds_without_touching_main() {
        let mut registry = Registry::new();
        for def in default_registry().definitions() {
            assert!(["bash", "read_file", "write_file"].contains(&def.name.as_str()));
        }
        registry.register(ExtraTool);
        let names: Vec<String> = registry.definitions().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["zzz_extra".to_string()]);
        let (text, is_err) = registry.execute("zzz_extra", "{}");
        assert!(!is_err);
        assert_eq!(text, "extra");
    }

    #[test]
    fn lesson_09_unknown_is_error_result() {
        let (text, is_err) = default_registry().execute("not_a_tool", "{}");
        assert!(is_err);
        assert!(text.contains("unknown tool"), "{text}");
    }
}
