//! 后学：[第十四讲 从单循环到图工程](https://walkinglabs.github.io/learn-harness-engineering/zh/lectures/lecture-14-graph-engineering/)
//!
//! 把 maker-checker 拆成显式节点和路由。不进 `agent_loop`，不引入 LangGraph。
//! verify 只看验证命令退出码；merge 前停住等人 `/graph approve`。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::loop_ctl::{
    arm_goal, is_placeholder_verify, maker_prompt, read_goal, GoalSpec, PLACEHOLDER_VERIFY, RUN_DIR,
};

pub const GRAPH_DIR: &str = ".local/graph";
pub const DEFAULT_THREAD: &str = "session-1";

const STATE_FILE: &str = "state.md";
const STOP_FILE: &str = "graph.stop";
const PID_FILE: &str = "graph.pid";
const LOG_FILE: &str = "graph.log";
const ACTIVE_FILE: &str = "active-thread";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNode {
    Implement,
    Verify,
    Pause,
    Merge,
    Done,
    Blocked,
}

impl GraphNode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implement => "implement",
            Self::Verify => "verify",
            Self::Pause => "pause",
            Self::Merge => "merge",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    fn parse(s: &str) -> Self {
        match s.trim() {
            "verify" => Self::Verify,
            "pause" => Self::Pause,
            "merge" => Self::Merge,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            _ => Self::Implement,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphState {
    pub thread: String,
    pub requirements: String,
    pub code: String,
    pub review: Option<bool>,
    pub attempts: u32,
    pub current: GraphNode,
    pub last_note: String,
}

impl GraphState {
    pub fn review_label(&self) -> &'static str {
        match self.review {
            Some(true) => "pass",
            Some(false) => "fail",
            None => "(none)",
        }
    }
}

pub fn parse_thread(raw: &str) -> Result<String, String> {
    let thread = raw.trim();
    if thread.is_empty() {
        return Ok(DEFAULT_THREAD.into());
    }
    if !thread
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("bad thread id: {thread}"));
    }
    Ok(thread.to_string())
}

pub fn thread_dir(root: &Path, thread: &str) -> PathBuf {
    root.join(GRAPH_DIR).join(thread)
}

pub fn parse_graph_state(text: &str) -> GraphState {
    let review = match section(text, "review").unwrap_or_default().as_str() {
        "pass" => Some(true),
        "fail" => Some(false),
        _ => None,
    };
    GraphState {
        thread: section(text, "thread").unwrap_or_else(|| DEFAULT_THREAD.into()),
        requirements: section(text, "requirements").unwrap_or_default(),
        code: section(text, "code").unwrap_or_default(),
        review,
        attempts: first_u32(&section(text, "attempts").unwrap_or_default()).unwrap_or(0),
        current: GraphNode::parse(&section(text, "current").unwrap_or_default()),
        last_note: section(text, "last_note").unwrap_or_default(),
    }
}

pub fn render_graph_state(state: &GraphState) -> String {
    format!(
        "# Graph checkpoint\n\n## thread\n{}\n\n## requirements\n{}\n\n## code\n{}\n\n## review\n{}\n\n## attempts\n{}\n\n## current\n{}\n\n## last_note\n{}\n",
        state.thread.trim(),
        state.requirements.trim(),
        state.code.trim(),
        state.review_label(),
        state.attempts,
        state.current.as_str(),
        state.last_note.trim()
    )
}

pub fn after_implement(state: &GraphState, notes: &str) -> GraphState {
    let mut next = state.clone();
    next.code = clip_tail(notes, 800);
    next.review = None;
    next.current = GraphNode::Verify;
    next.last_note = "implement 写完，交给 verify".into();
    next
}

pub fn after_verify(state: &GraphState, spec: &GoalSpec, exit_code: i32) -> GraphState {
    let mut next = state.clone();
    if exit_code == 0 {
        next.review = Some(true);
        next.current = GraphNode::Pause;
        next.last_note = "verify pass，pause_before merge".into();
        return next;
    }
    next.review = Some(false);
    next.attempts = next.attempts.saturating_add(1);
    if next.attempts >= spec.max_rounds {
        next.current = GraphNode::Blocked;
        next.last_note = format!("verify fail，已到最大回合 {}", spec.max_rounds);
    } else {
        next.current = GraphNode::Implement;
        next.last_note = "verify fail，回到 implement".into();
    }
    next
}

pub fn after_approve(state: &GraphState) -> Result<GraphState, String> {
    if state.current != GraphNode::Pause {
        return Err(format!(
            "approve only at pause (current={})",
            state.current.as_str()
        ));
    }
    let mut next = state.clone();
    next.current = GraphNode::Merge;
    next.last_note = "人批准，进入 merge".into();
    Ok(next)
}

pub fn after_merge(state: &GraphState) -> GraphState {
    let mut next = state.clone();
    next.current = GraphNode::Done;
    next.last_note = "merge 写完，未自动 git commit".into();
    next
}

pub fn should_run(state: &GraphState, stop: bool) -> bool {
    if stop {
        return false;
    }
    matches!(
        state.current,
        GraphNode::Implement | GraphNode::Verify | GraphNode::Merge
    )
}

pub fn arm_graph(root: &Path, thread: &str, objective: &str) -> Result<GraphState, String> {
    let thread = parse_thread(thread)?;
    let spec = arm_goal(root, objective)?;
    let state = GraphState {
        thread: thread.clone(),
        requirements: spec.objective,
        code: String::new(),
        review: None,
        attempts: 0,
        current: GraphNode::Implement,
        last_note: "武装，从 implement 开始".into(),
    };
    write_checkpoint(root, &state)?;
    set_active_thread(root, &thread)?;
    let _ = fs::remove_file(thread_dir(root, &thread).join(STOP_FILE));
    Ok(state)
}

pub fn read_checkpoint(root: &Path, thread: &str) -> Result<GraphState, String> {
    let path = thread_dir(root, thread).join(STATE_FILE);
    let text =
        fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    Ok(parse_graph_state(&text))
}

pub fn write_checkpoint(root: &Path, state: &GraphState) -> Result<(), String> {
    let dir = thread_dir(root, &state.thread);
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let path = dir.join(STATE_FILE);
    fs::write(&path, render_graph_state(state))
        .map_err(|err| format!("write {}: {err}", path.display()))
}

pub fn graph_stop_exists(root: &Path, thread: &str) -> bool {
    thread_dir(root, thread).join(STOP_FILE).exists()
}

pub fn write_graph_stop(root: &Path, thread: &str) -> Result<(), String> {
    let dir = thread_dir(root, thread);
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    fs::write(dir.join(STOP_FILE), "stop\n").map_err(|err| err.to_string())
}

pub fn active_thread(root: &Path) -> String {
    let path = root.join(GRAPH_DIR).join(ACTIVE_FILE);
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_THREAD.into())
}

pub fn set_active_thread(root: &Path, thread: &str) -> Result<(), String> {
    let dir = root.join(GRAPH_DIR);
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    fs::write(dir.join(ACTIVE_FILE), format!("{thread}\n")).map_err(|err| err.to_string())
}

pub fn graph_status_text(root: &Path, thread: &str) -> String {
    match read_checkpoint(root, thread) {
        Ok(state) => format!(
            "thread: {}\ncurrent: {}\nreview: {}\nattempts: {}\nrequirements: {}\nnext: {}\nlog: {}/{thread}/{LOG_FILE}\n",
            state.thread,
            state.current.as_str(),
            state.review_label(),
            state.attempts,
            state.requirements.trim(),
            state.last_note.trim(),
            GRAPH_DIR
        ),
        Err(_) => format!("no graph armed for {thread}. /graph <目标>\n"),
    }
}

pub fn graph_run_block_reason(root: &Path, thread: &str) -> Option<String> {
    let spec = match read_goal(root) {
        Ok(spec) => spec,
        Err(_) => return Some("先 /graph <目标>".into()),
    };
    if is_placeholder_verify(&spec.verify_command) {
        return Some(format!(
            "先把 {RUN_DIR}/goal.md 的验证命令从 {PLACEHOLDER_VERIFY} 改成可执行命令"
        ));
    }
    if read_checkpoint(root, thread).is_err() {
        return Some("先 /graph <目标>".into());
    }
    if thread_dir(root, thread).join(PID_FILE).exists() {
        return Some(format!(
            "graph runner already started, see {GRAPH_DIR}/{thread}/{LOG_FILE}"
        ));
    }
    None
}

pub fn graph_maker_prompt(root: &Path, thread: &str) -> Result<String, String> {
    let state = read_checkpoint(root, thread)?;
    let base = maker_prompt(root)?;
    Ok(format!(
        "{base}\n\n## 图节点\nimplement（私有上下文，不要自评通过）\n\n## 共享状态\n{}\n",
        render_graph_state(&state)
    ))
}

pub fn spawn_graph(root: &Path, thread: &str) -> Result<PathBuf, String> {
    let dir = thread_dir(root, thread);
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let log_path = dir.join(LOG_FILE);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| err.to_string())?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    Command::new("bash")
        .arg(root.join("scripts/graph-run.sh"))
        .current_dir(root)
        .env("GRAPH_THREAD", thread)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| format!("start scripts/graph-run.sh: {e} (need bash on PATH)"))?;
    Ok(log_path)
}

fn section(text: &str, heading: &str) -> Option<String> {
    let marker = format!("## {heading}");
    let rest = text.split(&marker).nth(1)?;
    let body = match rest.find("\n## ") {
        Some(i) => &rest[..i],
        None => rest,
    };
    Some(body.replace('\r', "").trim().to_string())
}

fn first_u32(text: &str) -> Option<u32> {
    let skip: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    skip.parse().ok()
}

fn clip_tail(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_string();
    }
    let skip = count - max_chars;
    format!("…{}", trimmed.chars().skip(skip).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loop_ctl::{render_goal, TEMPLATE_DIR};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("byo-graph-{nanos}"));
        fs::create_dir_all(root.join(TEMPLATE_DIR)).expect("templates");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1.0\"\n",
        )
        .expect("cargo");
        fs::write(
            root.join(TEMPLATE_DIR).join("goal.md"),
            render_goal(&GoalSpec {
                objective: "fix login".into(),
                verify_command: "true".into(),
                max_rounds: 2,
                constraints: "密钥".into(),
            }),
        )
        .expect("goal");
        fs::write(
            root.join(TEMPLATE_DIR).join("maker-prompt.md"),
            "你是 maker。\n",
        )
        .expect("maker");
        fs::write(
            root.join(TEMPLATE_DIR).join("checker-prompt.md"),
            "退出码。\n",
        )
        .expect("checker");
        fs::write(
            root.join(TEMPLATE_DIR).join("loop-state.md"),
            "# Loop state\n\n## 轮次\n0\n\n## 本轮做了什么\n-\n\n## 验证结果\n-\n\n## 状态\narmed\n\n## 下一步\n-\n",
        )
        .expect("state");
        root
    }

    #[test]
    fn route_fail_then_pass_then_approve() {
        let spec = GoalSpec {
            objective: "tests green".into(),
            verify_command: "true".into(),
            max_rounds: 2,
            constraints: String::new(),
        };
        let start = GraphState {
            thread: DEFAULT_THREAD.into(),
            requirements: spec.objective.clone(),
            code: String::new(),
            review: None,
            attempts: 0,
            current: GraphNode::Implement,
            last_note: String::new(),
        };
        let after_imp = after_implement(&start, "changed login");
        assert_eq!(after_imp.current, GraphNode::Verify);
        let failed = after_verify(&after_imp, &spec, 1);
        assert_eq!(failed.current, GraphNode::Implement);
        assert_eq!(failed.attempts, 1);
        assert!(should_run(&failed, false));
        let blocked = after_verify(&failed, &spec, 1);
        assert_eq!(blocked.current, GraphNode::Blocked);
        assert!(!should_run(&blocked, false));

        let passed = after_verify(&after_imp, &spec, 0);
        assert_eq!(passed.current, GraphNode::Pause);
        assert!(!should_run(&passed, false));
        let merge = after_approve(&passed).expect("approve");
        assert_eq!(merge.current, GraphNode::Merge);
        let done = after_merge(&merge);
        assert_eq!(done.current, GraphNode::Done);
        assert!(after_approve(&after_imp).is_err());
    }

    #[test]
    fn checkpoint_roundtrip() {
        let state = GraphState {
            thread: "t1".into(),
            requirements: "tests green".into(),
            code: "changed login".into(),
            review: Some(false),
            attempts: 1,
            current: GraphNode::Implement,
            last_note: "verify fail，回到 implement".into(),
        };
        let loaded = parse_graph_state(&render_graph_state(&state));
        assert_eq!(loaded, state);
    }

    #[test]
    fn arm_writes_checkpoint() {
        let root = tmp_root();
        let state = arm_graph(&root, DEFAULT_THREAD, "all tests pass").expect("arm");
        assert_eq!(state.current, GraphNode::Implement);
        assert_eq!(state.requirements, "all tests pass");
        let loaded = read_checkpoint(&root, DEFAULT_THREAD).expect("read");
        assert_eq!(loaded.thread, DEFAULT_THREAD);
        assert!(graph_run_block_reason(&root, DEFAULT_THREAD).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
