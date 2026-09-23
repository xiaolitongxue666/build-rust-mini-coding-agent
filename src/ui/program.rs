//! 第 12 课：[The full TUI](https://www.byoharness.dev/chapters/12-full-tui.html)
//!
//! 一个程序占整屏。课上是 Bubble Tea；本仓库继续第 08 课的 crossterm MVU，不上 ratatui。
//!
//! ```text
//! 空闲 ──提交──▶ 在跑 ──Confirm──▶ 等审批 ──y/n──▶ 在跑 ──结束──▶ 空闲
//! ```
//!
//! 循环在**后台线程**跑。`println!` 进管子变成 `Append`。
//! 审批：线程发 `Approval`，堵在通道上；主循环写回 true/false。
//! Update / apply 里不准 `println!`，否则字绕回自己。
//!
//! 底栏永远留 5 行，状态切换时布局不抖。

use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use crate::agent::Agent;
use crate::commands::{run_command, CommandCtx, CommandOutcome};
use crate::compact::NoCompaction;
use crate::provider::Provider;
use crate::subagent;
use crate::ui::chat_input::{
    append_history, load_history, render_box, ChatInputState, ChatKey, ChatOutcome,
};
use crate::ui::input::ctrl_c_action;
use crate::ui::stdout_pipe::StdoutCapture;
use crate::ui::{banner_text, term_width, CtrlCAction};

const BOTTOM_LINES: u16 = 5;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    Idle,
    Running,
    AwaitingApproval,
}

pub enum HarnessEvent {
    Key(ChatKey),
    PageUp,
    PageDown,
    Home,
    End,
    Resize { width: u16, height: u16 },
    Append(String),
    Approval { prompt: String },
    AgentDone { err: Option<String> },
    Tick,
    CtrlC,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessCmd {
    None,
    Submit(String),
    Quit,
    Reply(bool),
}

pub struct Harness {
    pub state: ModelState,
    pub output: String,
    pub follow_bottom: bool,
    pub scroll: usize,
    pub approval_prompt: String,
    pub input: ChatInputState,
    pub spinner_frame: usize,
    pub width: u16,
    pub height: u16,
    ctrl_c_streak: u8,
}

impl Harness {
    pub fn new(width: u16, height: u16, banner: String, history: Vec<String>) -> Self {
        let mut input = ChatInputState::new(width.max(24) as usize, history);
        input.set_width(width.max(24) as usize);
        Self {
            state: ModelState::Idle,
            output: banner,
            follow_bottom: true,
            scroll: 0,
            approval_prompt: String::new(),
            input,
            spinner_frame: 0,
            width,
            height,
            ctrl_c_streak: 0,
        }
    }

    pub fn at_bottom(&self) -> bool {
        self.scroll == 0
    }

    pub fn apply(&mut self, event: HarnessEvent) -> HarnessCmd {
        match event {
            HarnessEvent::Resize { width, height } => {
                self.width = width;
                self.height = height;
                self.input.set_width(width.max(24) as usize);
                if self.follow_bottom {
                    self.scroll = 0;
                }
                HarnessCmd::None
            }
            HarnessEvent::Append(text) => {
                self.output.push_str(&text);
                if self.follow_bottom {
                    self.scroll = 0;
                }
                HarnessCmd::None
            }
            HarnessEvent::Approval { prompt } => {
                self.state = ModelState::AwaitingApproval;
                self.approval_prompt = prompt;
                HarnessCmd::None
            }
            HarnessEvent::AgentDone { err } => {
                self.state = ModelState::Idle;
                if let Some(err) = err {
                    self.output.push_str(&format!("error: {err}\n"));
                    if self.follow_bottom {
                        self.scroll = 0;
                    }
                }
                HarnessCmd::None
            }
            HarnessEvent::Tick => {
                if self.state == ModelState::Running {
                    self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
                }
                HarnessCmd::None
            }
            HarnessEvent::PageUp => {
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll());
                self.follow_bottom = self.at_bottom();
                HarnessCmd::None
            }
            HarnessEvent::PageDown => {
                self.scroll = self.scroll.saturating_sub(1);
                self.follow_bottom = self.at_bottom();
                HarnessCmd::None
            }
            HarnessEvent::Home => {
                self.scroll = self.max_scroll();
                self.follow_bottom = false;
                HarnessCmd::None
            }
            HarnessEvent::End => {
                self.scroll = 0;
                self.follow_bottom = true;
                HarnessCmd::None
            }
            HarnessEvent::CtrlC => {
                self.ctrl_c_streak = self.ctrl_c_streak.saturating_add(1);
                match ctrl_c_action(self.ctrl_c_streak) {
                    CtrlCAction::Quit => HarnessCmd::Quit,
                    CtrlCAction::Clear => {
                        self.input.clear();
                        HarnessCmd::None
                    }
                }
            }
            HarnessEvent::Key(key) => self.apply_key(key),
        }
    }

    fn apply_key(&mut self, key: ChatKey) -> HarnessCmd {
        if self.state == ModelState::AwaitingApproval {
            return match key {
                ChatKey::Char('y' | 'Y') => {
                    self.state = ModelState::Running;
                    HarnessCmd::Reply(true)
                }
                ChatKey::Esc => {
                    self.state = ModelState::Running;
                    HarnessCmd::Reply(false)
                }
                ChatKey::Char('n' | 'N') | ChatKey::Enter => {
                    self.state = ModelState::Running;
                    HarnessCmd::Reply(false)
                }
                ChatKey::CtrlD => HarnessCmd::Quit,
                _ => HarnessCmd::None,
            };
        }
        if self.state != ModelState::Idle {
            return match key {
                ChatKey::CtrlD => HarnessCmd::Quit,
                _ => HarnessCmd::None,
            };
        }
        match self.input.apply(key) {
            ChatOutcome::Quit => HarnessCmd::Quit,
            ChatOutcome::Cleared => {
                self.ctrl_c_streak = 0;
                HarnessCmd::None
            }
            ChatOutcome::Continue => {
                self.ctrl_c_streak = 0;
                HarnessCmd::None
            }
            ChatOutcome::Submit(text) => {
                self.ctrl_c_streak = 0;
                let text = text.trim().to_string();
                if text.is_empty() {
                    return HarnessCmd::None;
                }
                append_history(&text);
                self.output
                    .push_str(&format!("\n{DIM}──{RESET}\n❯ {text}\n\n"));
                self.input.clear();
                self.follow_bottom = true;
                self.scroll = 0;
                self.state = ModelState::Running;
                HarnessCmd::Submit(text)
            }
        }
    }

    fn max_scroll(&self) -> usize {
        let vis = self.viewport_rows();
        self.output_lines().len().saturating_sub(vis)
    }

    fn viewport_rows(&self) -> usize {
        self.height.saturating_sub(BOTTOM_LINES) as usize
    }

    fn output_lines(&self) -> Vec<&str> {
        if self.output.is_empty() {
            return Vec::new();
        }
        self.output.lines().collect()
    }

    pub fn status_line(&self, active: &[(String, usize)]) -> String {
        if self.state != ModelState::Running {
            return " ".repeat(self.width.max(1) as usize);
        }
        let frame = SPINNER_FRAMES[self.spinner_frame];
        let mut line = format!("{frame} thinking...");
        for (name, n) in active {
            if *n == 0 {
                continue;
            }
            if *n == 1 {
                line.push_str(&format!(" · {name}"));
            } else {
                line.push_str(&format!(" · {name} ×{n}"));
            }
        }
        pad_line(&line, self.width as usize)
    }

    pub fn view(&self, active: &[(String, usize)]) -> String {
        let mut rows = Vec::new();
        let vis = self.viewport_rows().max(1);
        let lines = self.output_lines();
        let max_scroll = lines.len().saturating_sub(vis);
        let scroll = self.scroll.min(max_scroll);
        let end = lines.len().saturating_sub(scroll);
        let start = end.saturating_sub(vis);
        for line in lines.get(start..end).unwrap_or(&[]) {
            rows.push(pad_line(line, self.width as usize));
        }
        while rows.len() < vis {
            rows.insert(0, " ".repeat(self.width as usize));
        }
        rows.push(self.status_line(active));
        if self.state == ModelState::AwaitingApproval {
            rows.push(render_approval(&self.approval_prompt, self.width as usize));
        } else {
            rows.push(render_box(&self.input));
        }
        rows.join("\n")
    }
}

fn pad_line(line: &str, width: usize) -> String {
    let width = width.max(1);
    let visible = strip_ansi_len(line);
    if visible >= width {
        return line.chars().take(width).collect();
    }
    format!("{line}{}", " ".repeat(width - visible))
}

fn strip_ansi_len(s: &str) -> usize {
    let mut n = 0;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            n += 1;
        }
    }
    n
}

fn render_approval(prompt: &str, width: usize) -> String {
    let inner = (width.saturating_sub(2)).max(20);
    let body = format!("{prompt}  y/n");
    let mut line = body.chars().take(inner).collect::<String>();
    if strip_ansi_len(&line) < inner {
        line.push_str(&" ".repeat(inner - strip_ansi_len(&line)));
    }
    let bar = "─".repeat(inner);
    format!(
        "{YELLOW}╭{bar}╮{RESET}\n{YELLOW}│{RESET}{line}{YELLOW}│{RESET}\n{YELLOW}╰{bar}╯{RESET}\n{DIM} y: approve · n/esc: deny{RESET}"
    )
}

enum Job {
    Line(String),
    Quit,
}

enum UiMsg {
    Append(String),
    Approval {
        prompt: String,
        reply: SyncSender<bool>,
    },
    AgentDone {
        err: Option<String>,
    },
    Quit,
}

/// 第 12 课：备用屏 + 管子 + 工作线程。管道 / `BYO_PLAIN_INPUT` 不要走这里。
pub fn run_tui<P>(mut agent: Agent<P>, subagents: subagent::Registry) -> Result<(), String>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    let width = term_width().max(40) as u16;
    let height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    let banner = banner_text(width as usize);
    let history = load_history();

    let mut capture = StdoutCapture::install().map_err(|e| e.to_string())?;
    let (ui_tx, ui_rx) = mpsc::channel::<UiMsg>();
    let (job_tx, job_rx) = mpsc::channel::<Job>();

    {
        let ui_tx = ui_tx.clone();
        let reader = capture.take_reader().map_err(|e| e.to_string())?;
        thread::spawn(move || {
            let buf = BufReader::new(reader);
            for line in buf.lines() {
                match line {
                    Ok(text) => {
                        if ui_tx.send(UiMsg::Append(format!("{text}\n"))).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let confirm_tx = ui_tx.clone();
    agent.confirm = Some(Arc::new(move |prompt: &str| {
        let (rtx, rrx) = mpsc::sync_channel(1);
        if confirm_tx
            .send(UiMsg::Approval {
                prompt: prompt.to_string(),
                reply: rtx,
            })
            .is_err()
        {
            return false;
        }
        rrx.recv().unwrap_or(false)
    }));

    let worker_ui = ui_tx;
    thread::spawn(move || {
        let compact = NoCompaction;
        while let Ok(job) = job_rx.recv() {
            match job {
                Job::Quit => break,
                Job::Line(text) => {
                    let defs = agent.tools.definitions();
                    let mut verbose = agent.verbose;
                    let outcome = {
                        let mut ctx = CommandCtx {
                            llm: &mut agent.llm,
                            messages: &mut agent.messages,
                            tools: &defs,
                            compact: &compact,
                            verbose: &mut verbose,
                            subagents: &subagents,
                        };
                        run_command(&text, &mut ctx)
                    };
                    agent.verbose = verbose;
                    match outcome {
                        Some(CommandOutcome::Quit) => {
                            let _ = worker_ui.send(UiMsg::Quit);
                            break;
                        }
                        Some(CommandOutcome::Handled) => {
                            let _ = worker_ui.send(UiMsg::AgentDone { err: None });
                        }
                        None => {
                            let err = agent.send(text).err();
                            let _ = worker_ui.send(UiMsg::AgentDone { err });
                        }
                    }
                }
            }
        }
    });

    let mut out = capture.clone_original().map_err(|e| e.to_string())?;
    enable_raw_mode().map_err(|e| e.to_string())?;
    execute!(out, EnterAlternateScreen).map_err(|e| e.to_string())?;

    let mut harness = Harness::new(width, height, banner, history);
    let mut pending_reply: Option<SyncSender<bool>> = None;
    let result = event_loop(&mut out, &mut harness, &ui_rx, &job_tx, &mut pending_reply);

    let _ = job_tx.send(Job::Quit);
    let _ = execute!(out, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

fn event_loop(
    out: &mut FileWriter,
    harness: &mut Harness,
    ui_rx: &mpsc::Receiver<UiMsg>,
    job_tx: &Sender<Job>,
    pending_reply: &mut Option<SyncSender<bool>>,
) -> Result<(), String>
where
    FileWriter: Write,
{
    draw(out, harness)?;
    loop {
        while let Ok(msg) = ui_rx.try_recv() {
            match msg {
                UiMsg::Append(text) => {
                    let _ = harness.apply(HarnessEvent::Append(text));
                }
                UiMsg::Approval { prompt, reply } => {
                    *pending_reply = Some(reply);
                    let _ = harness.apply(HarnessEvent::Approval { prompt });
                }
                UiMsg::AgentDone { err } => {
                    let _ = harness.apply(HarnessEvent::AgentDone { err });
                }
                UiMsg::Quit => return Ok(()),
            }
        }

        if event::poll(Duration::from_millis(80)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Resize(width, height) => {
                    let _ = harness.apply(HarnessEvent::Resize { width, height });
                }
                Event::Key(key)
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
                {
                    match dispatch_key(harness, key, job_tx, pending_reply) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {}
                        Err(err) => return Err(err),
                    }
                }
                _ => {}
            }
        } else {
            let _ = harness.apply(HarnessEvent::Tick);
        }
        draw(out, harness)?;
    }
}

fn dispatch_key(
    harness: &mut Harness,
    key: KeyEvent,
    job_tx: &Sender<Job>,
    pending_reply: &mut Option<SyncSender<bool>>,
) -> Result<bool, String> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return finish_cmd(harness.apply(HarnessEvent::CtrlC), job_tx, pending_reply);
    }
    let ev = match key.code {
        KeyCode::PageUp => HarnessEvent::PageUp,
        KeyCode::PageDown => HarnessEvent::PageDown,
        KeyCode::Home if harness.state != ModelState::Idle => HarnessEvent::Home,
        KeyCode::End
            if harness.state != ModelState::Idle
                || key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            HarnessEvent::End
        }
        other => match key_to_chat(other, key.modifiers) {
            Some(k) => HarnessEvent::Key(k),
            None => return Ok(false),
        },
    };
    finish_cmd(harness.apply(ev), job_tx, pending_reply)
}

fn finish_cmd(
    cmd: HarnessCmd,
    job_tx: &Sender<Job>,
    pending_reply: &mut Option<SyncSender<bool>>,
) -> Result<bool, String> {
    match cmd {
        HarnessCmd::None => Ok(false),
        HarnessCmd::Quit => {
            let _ = job_tx.send(Job::Quit);
            Ok(true)
        }
        HarnessCmd::Submit(text) => {
            job_tx.send(Job::Line(text)).map_err(|e| e.to_string())?;
            Ok(false)
        }
        HarnessCmd::Reply(yes) => {
            if let Some(reply) = pending_reply.take() {
                let _ = reply.send(yes);
            }
            Ok(false)
        }
    }
}

fn key_to_chat(code: KeyCode, mods: KeyModifiers) -> Option<ChatKey> {
    Some(match code {
        KeyCode::Enter => ChatKey::Enter,
        KeyCode::Esc => ChatKey::Esc,
        KeyCode::Backspace => ChatKey::Backspace,
        KeyCode::Delete => ChatKey::Delete,
        KeyCode::Left => ChatKey::Left,
        KeyCode::Right => ChatKey::Right,
        KeyCode::Up => ChatKey::Up,
        KeyCode::Down => ChatKey::Down,
        KeyCode::Home => ChatKey::Home,
        KeyCode::End => ChatKey::End,
        KeyCode::Char('d') if mods.contains(KeyModifiers::CONTROL) => ChatKey::CtrlD,
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => ChatKey::CtrlC,
        KeyCode::Char(c) => ChatKey::Char(c),
        _ => return None,
    })
}

fn active_pairs() -> Vec<(String, usize)> {
    let mut pairs: Vec<(String, usize)> = subagent::active().into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn draw(out: &mut impl Write, harness: &Harness) -> Result<(), String> {
    let view = harness.view(&active_pairs());
    queue!(out, MoveTo(0, 0), Clear(ClearType::All)).map_err(|e| e.to_string())?;
    write!(out, "{view}").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

type FileWriter = std::fs::File;

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Harness {
        Harness::new(40, 12, "banner\n".into(), Vec::new())
    }

    #[test]
    fn submit_moves_idle_to_running() {
        let mut h = small();
        assert_eq!(h.state, ModelState::Idle);
        let _ = h.apply(HarnessEvent::Key(ChatKey::Char('h')));
        let _ = h.apply(HarnessEvent::Key(ChatKey::Char('i')));
        let cmd = h.apply(HarnessEvent::Key(ChatKey::Enter));
        assert_eq!(cmd, HarnessCmd::Submit("hi".into()));
        assert_eq!(h.state, ModelState::Running);
        assert!(h.output.contains("❯ hi"), "{}", h.output);
        assert!(h.follow_bottom);
    }

    #[test]
    fn approval_then_yes_replies_and_runs() {
        let mut h = small();
        h.state = ModelState::Running;
        let _ = h.apply(HarnessEvent::Approval {
            prompt: "approve?".into(),
        });
        assert_eq!(h.state, ModelState::AwaitingApproval);
        let cmd = h.apply(HarnessEvent::Key(ChatKey::Char('y')));
        assert_eq!(cmd, HarnessCmd::Reply(true));
        assert_eq!(h.state, ModelState::Running);
    }

    #[test]
    fn page_up_stops_follow_end_resumes() {
        let mut h = small();
        let _ = h.apply(HarnessEvent::Append(
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n".into(),
        ));
        assert!(h.follow_bottom);
        let _ = h.apply(HarnessEvent::PageUp);
        assert!(!h.follow_bottom);
        let _ = h.apply(HarnessEvent::End);
        assert!(h.follow_bottom);
        assert_eq!(h.scroll, 0);
    }

    #[test]
    fn done_returns_idle() {
        let mut h = small();
        h.state = ModelState::Running;
        let _ = h.apply(HarnessEvent::AgentDone { err: None });
        assert_eq!(h.state, ModelState::Idle);
    }

    #[test]
    fn status_shows_subagent_when_running() {
        let mut h = small();
        h.state = ModelState::Running;
        let line = h.status_line(&[("research".into(), 1)]);
        assert!(line.contains("thinking..."), "{line}");
        assert!(line.contains("research"), "{line}");
    }

    #[test]
    fn approval_view_is_yellow() {
        let mut h = small();
        h.state = ModelState::AwaitingApproval;
        h.approval_prompt = "approve?".into();
        let view = h.view(&[]);
        assert!(view.contains("approve?"), "{view}");
        assert!(view.contains('╭'), "{view}");
    }
}
