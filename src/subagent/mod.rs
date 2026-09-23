//! 第 11 课：[Subagents](https://www.byoharness.dev/chapters/11-subagents.html)
//!
//! 和 Provider / Tool / CompactionStrategy 同一形状：一个接口，多份实现。
//! 子 agent 需要 Provider 和工具子集，**不能**像工具那样 `OnceLock` 自登记。
//! 在 REPL 接线里 `register`，再给每个挂一把 `DelegateTool`。
//!
//! `Begin` / `Active` 给 `/subagents` 和以后的状态栏。REPL 堵住时看不到飞行中的，
//! 第 12 课 TUI 才实时显示。本课只把计数器摆好。

mod research;

pub use research::Research;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub trait Subagent: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> String;
    fn run(&self, task: &str) -> Result<String, String>;
}

pub struct Registry {
    subagents: HashMap<String, Arc<dyn Subagent>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            subagents: HashMap::new(),
        }
    }

    pub fn register(&mut self, subagent: impl Subagent + 'static) {
        let boxed: Arc<dyn Subagent> = Arc::new(subagent);
        self.subagents.insert(boxed.name().to_string(), boxed);
    }

    pub fn all(&self) -> Vec<Arc<dyn Subagent>> {
        let mut names: Vec<&String> = self.subagents.keys().collect();
        names.sort();
        names
            .into_iter()
            .map(|name| self.subagents[name].clone())
            .collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

struct Tracker {
    active: HashMap<String, usize>,
}

fn tracker() -> &'static Mutex<Tracker> {
    static TRACKER: OnceLock<Mutex<Tracker>> = OnceLock::new();
    TRACKER.get_or_init(|| {
        Mutex::new(Tracker {
            active: HashMap::new(),
        })
    })
}

/// 第 11 课：`Run` 开头 `let _end = begin(name)`。Drop 时减计数。
pub fn begin(name: impl Into<String>) -> ActiveGuard {
    let name = name.into();
    if let Ok(mut state) = tracker().lock() {
        *state.active.entry(name.clone()).or_insert(0) += 1;
    }
    ActiveGuard { name }
}

pub fn active() -> HashMap<String, usize> {
    tracker()
        .lock()
        .map(|state| state.active.clone())
        .unwrap_or_default()
}

pub struct ActiveGuard {
    name: String,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = tracker().lock() {
            let remove = match state.active.get_mut(&self.name) {
                Some(count) => {
                    *count = count.saturating_sub(1);
                    *count == 0
                }
                None => false,
            };
            if remove {
                state.active.remove(&self.name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Subagent for Fake {
        fn name(&self) -> &str {
            "fake"
        }
        fn description(&self) -> String {
            "d".into()
        }
        fn run(&self, task: &str) -> Result<String, String> {
            Ok(task.to_string())
        }
    }

    #[test]
    fn all_is_sorted_by_name() {
        let mut registry = Registry::new();
        registry.register(Fake);
        let names: Vec<String> = registry
            .all()
            .iter()
            .map(|s| s.name().to_string())
            .collect();
        assert_eq!(names, vec!["fake".to_string()]);
    }

    #[test]
    fn begin_increments_until_drop() {
        assert!(!active().contains_key("tracker-test"));
        {
            let _g = begin("tracker-test");
            assert_eq!(active().get("tracker-test").copied(), Some(1));
        }
        assert!(!active().contains_key("tracker-test"));
    }
}
