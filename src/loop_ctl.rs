//! 后学：[第十三讲 从手动驱动到自动循环](https://walkinglabs.github.io/learn-harness-engineering/zh/lectures/lecture-13-loop-engineering/)
//!
//! 外层循环：目标、验证命令、停止条件。不进 `agent_loop`。
//! 验收只看验证命令退出码，不让写代码的那次模型调用给自己打分。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const TEMPLATE_DIR: &str = "loops";
pub const RUN_DIR: &str = ".local/loop";
pub const PLACEHOLDER_VERIFY: &str = "REPLACE_ME";

const GOAL_FILE: &str = "goal.md";
const STATE_FILE: &str = "loop-state.md";
const MAKER_FILE: &str = "maker-prompt.md";
const CHECKER_FILE: &str = "checker-prompt.md";
const PATROL_FILE: &str = "patrol.md";
const GOAL_STOP: &str = "goal.stop";
const PATROL_STOP: &str = "patrol.stop";
const GOAL_PID: &str = "goal.pid";
const PATROL_PID: &str = "patrol.pid";
const GOAL_LOG: &str = "goal.log";
const PATROL_LOG: &str = "patrol.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStatus {
    Armed,
    Passed,
    Failed,
    Blocked,
    Stopped,
}

impl RoundStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Armed => "armed",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Stopped => "stopped",
        }
    }

    fn parse(s: &str) -> Self {
        match s.trim() {
            "passed" => Self::Passed,
            "failed" => Self::Failed,
            "blocked" => Self::Blocked,
            "stopped" => Self::Stopped,
            _ => Self::Armed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalSpec {
    pub objective: String,
    pub verify_command: String,
    pub max_rounds: u32,
    pub constraints: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopState {
    pub round: u32,
    pub last_action: String,
    pub last_verify: String,
    pub status: RoundStatus,
    pub next: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatrolSpec {
    pub interval: String,
    pub interval_secs: u64,
    pub prompt: String,
}

pub fn find_root(start: &Path) -> Result<PathBuf, String> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(TEMPLATE_DIR).join(GOAL_FILE).is_file() && dir.join("Cargo.toml").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("not inside this harness repo (missing loops/goal.md)".into());
        }
    }
}

pub fn cwd_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    find_root(&cwd)
}

pub fn run_dir(root: &Path) -> PathBuf {
    root.join(RUN_DIR)
}

pub fn parse_interval(raw: &str) -> Result<u64, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("missing interval".into());
    }
    let split = raw.find(|c: char| !c.is_ascii_digit());
    let Some(i) = split else {
        return Err(format!("interval needs a unit: {raw}"));
    };
    let (num, unit) = raw.split_at(i);
    let n: u64 = num.parse().map_err(|_| format!("bad interval: {raw}"))?;
    if n == 0 {
        return Err("interval must be > 0".into());
    }
    match unit {
        "s" | "sec" | "secs" => Ok(n),
        "m" | "min" | "mins" => Ok(n.saturating_mul(60)),
        "h" | "hr" | "hrs" => Ok(n.saturating_mul(3600)),
        "d" | "day" | "days" => Ok(n.saturating_mul(86400)),
        _ => Err(format!("unknown interval unit: {unit}")),
    }
}

pub fn is_placeholder_verify(cmd: &str) -> bool {
    let text = cmd.trim();
    text.is_empty() || text == PLACEHOLDER_VERIFY || text.contains("占位") || text.starts_with('（')
}

pub fn parse_goal_md(text: &str) -> Result<GoalSpec, String> {
    let objective = section(text, "目标").unwrap_or_default();
    let verify_command = section(text, "验证命令").unwrap_or_default();
    let max_raw = section(text, "最大回合").unwrap_or_default();
    let constraints = section(text, "不能碰").unwrap_or_default();
    let max_rounds = first_u32(&max_raw).unwrap_or(8);
    if max_rounds == 0 {
        return Err("max rounds must be > 0".into());
    }
    Ok(GoalSpec {
        objective,
        verify_command,
        max_rounds,
        constraints,
    })
}

pub fn parse_state_md(text: &str) -> Result<LoopState, String> {
    let round = first_u32(&section(text, "轮次").unwrap_or_default()).unwrap_or(0);
    Ok(LoopState {
        round,
        last_action: section(text, "本轮做了什么").unwrap_or_default(),
        last_verify: section(text, "验证结果").unwrap_or_default(),
        status: RoundStatus::parse(&section(text, "状态").unwrap_or_default()),
        next: section(text, "下一步").unwrap_or_default(),
    })
}

pub fn parse_patrol_md(text: &str) -> Result<PatrolSpec, String> {
    let interval = section(text, "间隔").unwrap_or_default();
    let prompt = section(text, "巡检").unwrap_or_default();
    let interval_secs = parse_interval(&interval)?;
    if prompt.is_empty() || prompt.starts_with('（') {
        return Err("patrol prompt is empty".into());
    }
    Ok(PatrolSpec {
        interval,
        interval_secs,
        prompt,
    })
}

pub fn render_goal(spec: &GoalSpec) -> String {
    format!(
        "# Goal\n\n## 目标\n{}\n\n## 验证命令\n{}\n\n## 最大回合\n{}\n\n## 不能碰\n{}\n",
        spec.objective.trim(),
        spec.verify_command.trim(),
        spec.max_rounds,
        spec.constraints.trim()
    )
}

pub fn render_state(state: &LoopState) -> String {
    format!(
        "# Loop state\n\n## 轮次\n{}\n\n## 本轮做了什么\n{}\n\n## 验证结果\n{}\n\n## 状态\n{}\n\n## 下一步\n{}\n",
        state.round,
        state.last_action.trim(),
        state.last_verify.trim(),
        state.status.as_str(),
        state.next.trim()
    )
}

pub fn render_patrol(spec: &PatrolSpec) -> String {
    format!(
        "# Patrol\n\n## 间隔\n{}\n\n## 巡检\n{}\n",
        spec.interval.trim(),
        spec.prompt.trim()
    )
}

pub fn advance(
    state: &LoopState,
    spec: &GoalSpec,
    verify_exit: i32,
    maker_notes: &str,
) -> LoopState {
    let round = state.round.saturating_add(1);
    let last_action = clip_tail(maker_notes, 800);
    if verify_exit == 0 {
        return LoopState {
            round,
            last_action,
            last_verify: format!("exit {verify_exit}"),
            status: RoundStatus::Passed,
            next: "目标已达成".into(),
        };
    }
    if round >= spec.max_rounds {
        return LoopState {
            round,
            last_action,
            last_verify: format!("exit {verify_exit}"),
            status: RoundStatus::Blocked,
            next: format!("已到最大回合 {}", spec.max_rounds),
        };
    }
    LoopState {
        round,
        last_action,
        last_verify: format!("exit {verify_exit}"),
        status: RoundStatus::Failed,
        next: "按验证失败继续改".into(),
    }
}

pub fn should_continue(state: &LoopState, spec: &GoalSpec, stop: bool) -> bool {
    if stop {
        return false;
    }
    match state.status {
        RoundStatus::Passed | RoundStatus::Blocked | RoundStatus::Stopped => false,
        RoundStatus::Armed | RoundStatus::Failed => state.round < spec.max_rounds,
    }
}

pub fn arm_goal(root: &Path, objective: &str) -> Result<GoalSpec, String> {
    let objective = objective.trim();
    if objective.is_empty() {
        return Err("empty goal".into());
    }
    let templates = root.join(TEMPLATE_DIR);
    let run = run_dir(root);
    ensure_dir(&run)?;
    let dest = run.join(GOAL_FILE);
    if !dest.exists() {
        copy_file(&templates.join(GOAL_FILE), &dest)?;
    }
    copy_if_missing(&templates.join(MAKER_FILE), &run.join(MAKER_FILE))?;
    copy_if_missing(&templates.join(CHECKER_FILE), &run.join(CHECKER_FILE))?;
    copy_file(&templates.join(STATE_FILE), &run.join(STATE_FILE))?;

    let mut spec = parse_goal_md(&read_to_string(&dest)?)?;
    spec.objective = objective.to_string();
    write_string(&dest, &render_goal(&spec))?;

    let mut state = parse_state_md(&read_to_string(&run.join(STATE_FILE))?)?;
    state.round = 0;
    state.status = RoundStatus::Armed;
    state.last_action = "（尚未开始）".into();
    state.last_verify = "（尚未验证）".into();
    state.next = if is_placeholder_verify(&spec.verify_command) {
        "先把验证命令改成可执行命令，再 /goal run".into()
    } else {
        "执行 /goal run".into()
    };
    write_string(&run.join(STATE_FILE), &render_state(&state))?;
    let _ = fs::remove_file(run.join(GOAL_STOP));
    Ok(spec)
}

pub fn arm_patrol(root: &Path, interval: &str, prompt: &str) -> Result<PatrolSpec, String> {
    let interval_secs = parse_interval(interval)?;
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("empty patrol".into());
    }
    let run = run_dir(root);
    ensure_dir(&run)?;
    let spec = PatrolSpec {
        interval: interval.trim().to_string(),
        interval_secs,
        prompt: prompt.to_string(),
    };
    write_string(&run.join(PATROL_FILE), &render_patrol(&spec))?;
    let _ = fs::remove_file(run.join(PATROL_STOP));
    Ok(spec)
}

pub fn write_stop(root: &Path, kind: LoopKind) -> Result<(), String> {
    let run = run_dir(root);
    ensure_dir(&run)?;
    let name = match kind {
        LoopKind::Goal => GOAL_STOP,
        LoopKind::Patrol => PATROL_STOP,
    };
    write_string(&run.join(name), "stop\n")
}

pub fn goal_stop_exists(root: &Path) -> bool {
    run_dir(root).join(GOAL_STOP).exists()
}

pub fn patrol_stop_exists(root: &Path) -> bool {
    run_dir(root).join(PATROL_STOP).exists()
}

pub fn read_goal(root: &Path) -> Result<GoalSpec, String> {
    parse_goal_md(&read_to_string(&run_dir(root).join(GOAL_FILE))?)
}

pub fn read_state(root: &Path) -> Result<LoopState, String> {
    parse_state_md(&read_to_string(&run_dir(root).join(STATE_FILE))?)
}

pub fn write_state(root: &Path, state: &LoopState) -> Result<(), String> {
    write_string(&run_dir(root).join(STATE_FILE), &render_state(state))
}

pub fn read_patrol(root: &Path) -> Result<PatrolSpec, String> {
    parse_patrol_md(&read_to_string(&run_dir(root).join(PATROL_FILE))?)
}

pub fn maker_prompt(root: &Path) -> Result<String, String> {
    let run = run_dir(root);
    let spec = read_goal(root)?;
    let maker = read_to_string(&run.join(MAKER_FILE))
        .unwrap_or_else(|_| "你是 maker。读目标和上一轮状态，做一轮具体改动。".into());
    let state = read_to_string(&run.join(STATE_FILE)).unwrap_or_default();
    Ok(format!(
        "{}\n\n## 当前目标\n{}\n\n## 不能碰\n{}\n\n## 上一轮状态\n{}\n",
        maker.trim(),
        spec.objective.trim(),
        spec.constraints.trim(),
        state.trim()
    ))
}

pub fn goal_status_text(root: &Path) -> String {
    match (read_goal(root), read_state(root)) {
        (Ok(spec), Ok(state)) => {
            let verify = if is_placeholder_verify(&spec.verify_command) {
                format!("{} (占位，/goal run 会拒绝)", spec.verify_command.trim())
            } else {
                spec.verify_command.trim().to_string()
            };
            format!(
                "goal: {}\nverify: {verify}\nmax: {}\nround: {}/{}\nstatus: {}\nnext: {}\nlog: {}/{GOAL_LOG}\n",
                spec.objective.trim(),
                spec.max_rounds,
                state.round,
                spec.max_rounds,
                state.status.as_str(),
                state.next.trim(),
                RUN_DIR
            )
        }
        _ => "no goal armed. /goal <目标>\n".into(),
    }
}

pub fn patrol_status_text(root: &Path) -> String {
    match read_patrol(root) {
        Ok(spec) => format!(
            "interval: {} ({}s)\npatrol: {}\nlog: {}/{PATROL_LOG}\n",
            spec.interval,
            spec.interval_secs,
            spec.prompt.trim(),
            RUN_DIR
        ),
        Err(_) => "no patrol armed. /loop <间隔> <巡检>\n".into(),
    }
}

pub fn goal_run_block_reason(root: &Path) -> Option<String> {
    let spec = match read_goal(root) {
        Ok(spec) => spec,
        Err(_) => return Some("先 /goal <目标>".into()),
    };
    if is_placeholder_verify(&spec.verify_command) {
        return Some(format!(
            "先把 {RUN_DIR}/{GOAL_FILE} 的验证命令从 {PLACEHOLDER_VERIFY} 改成可执行命令"
        ));
    }
    if run_dir(root).join(GOAL_PID).exists() {
        return Some(format!(
            "goal runner already started, see {RUN_DIR}/{GOAL_LOG}"
        ));
    }
    None
}

pub fn patrol_run_block_reason(root: &Path) -> Option<String> {
    if read_patrol(root).is_err() {
        return Some("先 /loop <间隔> <巡检>".into());
    }
    if run_dir(root).join(PATROL_PID).exists() {
        return Some(format!(
            "patrol runner already started, see {RUN_DIR}/{PATROL_LOG}"
        ));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    Goal,
    Patrol,
}

pub fn spawn_runner(root: &Path, kind: LoopKind) -> Result<PathBuf, String> {
    let (script, log_name) = match kind {
        LoopKind::Goal => ("scripts/goal-run.sh", GOAL_LOG),
        LoopKind::Patrol => ("scripts/loop-tick.sh", PATROL_LOG),
    };
    let run = run_dir(root);
    ensure_dir(&run)?;
    let log_path = run.join(log_name);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| err.to_string())?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    Command::new("bash")
        .arg(root.join(script))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| format!("start {script}: {e} (need bash on PATH)"))?;
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
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        let skip: String = text
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        return skip.parse().ok();
    }
    digits.parse().ok()
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

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| err.to_string())
}

fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        ensure_dir(parent)?;
    }
    fs::copy(from, to).map_err(|err| format!("copy {}: {err}", from.display()))?;
    Ok(())
}

fn copy_if_missing(from: &Path, to: &Path) -> Result<(), String> {
    if to.exists() {
        return Ok(());
    }
    copy_file(from, to)
}

fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))
}

fn write_string(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, text).map_err(|err| format!("write {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("byo-loop-{nanos}"));
        fs::create_dir_all(root.join(TEMPLATE_DIR)).expect("templates");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1.0\"\n",
        )
        .expect("cargo");
        fs::write(
            root.join(TEMPLATE_DIR).join(GOAL_FILE),
            render_goal(&GoalSpec {
                objective: "（/goal 武装时写入）".into(),
                verify_command: PLACEHOLDER_VERIFY.into(),
                max_rounds: 8,
                constraints: "密钥".into(),
            }),
        )
        .expect("goal");
        fs::write(root.join(TEMPLATE_DIR).join(MAKER_FILE), "你是 maker。\n").expect("maker");
        fs::write(
            root.join(TEMPLATE_DIR).join(CHECKER_FILE),
            "退出码 0 算通过。\n",
        )
        .expect("checker");
        fs::write(
            root.join(TEMPLATE_DIR).join(STATE_FILE),
            render_state(&LoopState {
                round: 0,
                last_action: "（尚未开始）".into(),
                last_verify: "（尚未验证）".into(),
                status: RoundStatus::Armed,
                next: "再 /goal run".into(),
            }),
        )
        .expect("state");
        root
    }

    #[test]
    fn parse_interval_units() {
        assert_eq!(parse_interval("30s").expect("s"), 30);
        assert_eq!(parse_interval("15m").expect("m"), 900);
        assert_eq!(parse_interval("2h").expect("h"), 7200);
        assert_eq!(parse_interval("1d").expect("d"), 86400);
        assert!(parse_interval("15").is_err());
        assert!(parse_interval("0m").is_err());
    }

    #[test]
    fn placeholder_blocks_run() {
        assert!(is_placeholder_verify(PLACEHOLDER_VERIFY));
        assert!(is_placeholder_verify("（占位）"));
        assert!(!is_placeholder_verify("bash scripts/check.sh"));
    }

    #[test]
    fn advance_pass_fail_and_max() {
        let spec = GoalSpec {
            objective: "tests green".into(),
            verify_command: "true".into(),
            max_rounds: 2,
            constraints: String::new(),
        };
        let start = LoopState {
            round: 0,
            last_action: String::new(),
            last_verify: String::new(),
            status: RoundStatus::Armed,
            next: String::new(),
        };
        assert!(should_continue(&start, &spec, false));
        let failed = advance(&start, &spec, 1, "tried");
        assert_eq!(failed.status, RoundStatus::Failed);
        assert_eq!(failed.round, 1);
        assert!(should_continue(&failed, &spec, false));
        let blocked = advance(&failed, &spec, 1, "still failing");
        assert_eq!(blocked.status, RoundStatus::Blocked);
        assert!(!should_continue(&blocked, &spec, false));
        let passed = advance(&start, &spec, 0, "done");
        assert_eq!(passed.status, RoundStatus::Passed);
        assert!(!should_continue(&passed, &spec, false));
        assert!(!should_continue(&start, &spec, true));
    }

    #[test]
    fn arm_goal_writes_run_copy() {
        let root = tmp_root();
        let spec = arm_goal(&root, "all tests pass").expect("arm");
        assert_eq!(spec.objective, "all tests pass");
        assert!(is_placeholder_verify(&spec.verify_command));
        assert!(goal_run_block_reason(&root)
            .expect("block")
            .contains(PLACEHOLDER_VERIFY));

        let mut ready = read_goal(&root).expect("goal");
        ready.verify_command = "true".into();
        write_string(&run_dir(&root).join(GOAL_FILE), &render_goal(&ready)).expect("write");
        assert!(goal_run_block_reason(&root).is_none());

        let state = read_state(&root).expect("state");
        assert_eq!(state.round, 0);
        assert_eq!(state.status, RoundStatus::Armed);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn arm_patrol_keeps_interval() {
        let root = tmp_root();
        let spec = arm_patrol(&root, "15m", "跑测试，失败只报告").expect("arm");
        assert_eq!(spec.interval_secs, 900);
        let loaded = read_patrol(&root).expect("read");
        assert_eq!(loaded.prompt, "跑测试，失败只报告");
        let _ = fs::remove_dir_all(&root);
    }
}
