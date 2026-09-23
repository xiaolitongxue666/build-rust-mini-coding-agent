//! 第 04 课指定键位：↑↓ 历史、Ctrl+C 一次清空、两次静默退出。
//! 第 08 课：TTY 走一次性边框输入（crossterm MVU），confirm 共用同一条读键路。
//! 不要再 lock 一份 stdin。历史写入当前 `$HOME/.rustbyo_harness_history`（会留下贴进去的密钥）。

use std::io::{self, Write};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{cursor, queue};

use super::banner::term_width;
use super::chat_input::{
    append_history, load_history, render_box, ChatInputState, ChatKey, ChatOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlCAction {
    Clear,
    Quit,
}

/// 连续 Interrupted 次数（含这一次）。一次清空，两次退出。
pub fn ctrl_c_action(streak: u8) -> CtrlCAction {
    if streak >= 2 {
        CtrlCAction::Quit
    } else {
        CtrlCAction::Clear
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineResult {
    Line(String),
    End,
    Abort,
}

pub trait PromptRead {
    fn read_line(&mut self, prompt: &str) -> LineResult;
}

impl<I> PromptRead for I
where
    I: Iterator<Item = io::Result<String>>,
{
    fn read_line(&mut self, prompt: &str) -> LineResult {
        if !prompt.is_empty() {
            print!("{prompt}");
            let _ = io::stdout().flush();
        }
        match self.next() {
            Some(Ok(line)) => LineResult::Line(line),
            _ => LineResult::End,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplLine {
    Text(String),
    Quit,
}

pub struct SessionInput {
    ctrl_c_streak: u8,
}

impl SessionInput {
    pub fn new() -> Result<Self, String> {
        Ok(Self { ctrl_c_streak: 0 })
    }

    pub fn read_repl(&mut self) -> ReplLine {
        read_chat_input(&mut self.ctrl_c_streak)
    }
}

impl PromptRead for SessionInput {
    fn read_line(&mut self, prompt: &str) -> LineResult {
        read_confirm_line(prompt)
    }
}

struct RawGuard;

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn read_chat_input(streak: &mut u8) -> ReplLine {
    let width = match term_width() {
        0 => 80,
        w => w,
    };
    let mut state = ChatInputState::new(width, load_history());
    if enable_raw_mode().is_err() {
        return ReplLine::Quit;
    }
    let _raw = RawGuard;
    let mut first = true;
    loop {
        if draw_box(&state, first).is_err() {
            return ReplLine::Quit;
        }
        first = false;
        let Some(key) = read_key() else {
            return ReplLine::Quit;
        };
        match state.apply(key) {
            ChatOutcome::Continue => *streak = 0,
            ChatOutcome::Cleared => {
                *streak = streak.saturating_add(1);
                if ctrl_c_action(*streak) == CtrlCAction::Quit {
                    return ReplLine::Quit;
                }
            }
            ChatOutcome::Submit(text) => {
                let input = text.trim();
                if input.is_empty() {
                    continue;
                }
                append_history(input);
                let _ = writeln!(io::stdout());
                return ReplLine::Text(input.to_string());
            }
            ChatOutcome::Quit => return ReplLine::Quit,
        }
    }
}

fn read_confirm_line(prompt: &str) -> LineResult {
    if enable_raw_mode().is_err() {
        return LineResult::End;
    }
    let _raw = RawGuard;
    let _ = write!(io::stdout(), "{prompt}");
    let _ = io::stdout().flush();
    loop {
        match read_key() {
            Some(ChatKey::Esc) => return LineResult::Abort,
            Some(ChatKey::CtrlD) | Some(ChatKey::CtrlC) => return LineResult::End,
            Some(ChatKey::Enter) => return LineResult::Line(String::new()),
            Some(ChatKey::Char(c)) => {
                let _ = writeln!(io::stdout(), "{c}");
                return LineResult::Line(c.to_string());
            }
            Some(_) => {}
            None => return LineResult::End,
        }
    }
}

fn draw_box(state: &ChatInputState, first: bool) -> io::Result<()> {
    let view = render_box(state);
    let rows = view.lines().count() as u16;
    let mut out = io::stdout();
    if !first {
        queue!(
            out,
            cursor::MoveToPreviousLine(rows),
            cursor::MoveToColumn(0),
            Clear(ClearType::FromCursorDown)
        )?;
    }
    queue!(out, cursor::MoveToColumn(0))?;
    write!(out, "{view}")?;
    out.flush()
}

fn read_key() -> Option<ChatKey> {
    loop {
        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => return map_key(key),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

fn map_key(key: KeyEvent) -> Option<ChatKey> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => Some(ChatKey::CtrlC),
            KeyCode::Char('d') | KeyCode::Char('D') => Some(ChatKey::CtrlD),
            KeyCode::Char('a') | KeyCode::Char('A') => Some(ChatKey::Home),
            KeyCode::Char('e') | KeyCode::Char('E') => Some(ChatKey::End),
            _ => None,
        };
    }
    Some(match key.code {
        KeyCode::Enter => ChatKey::Enter,
        KeyCode::Up => ChatKey::Up,
        KeyCode::Down => ChatKey::Down,
        KeyCode::Left => ChatKey::Left,
        KeyCode::Right => ChatKey::Right,
        KeyCode::Home => ChatKey::Home,
        KeyCode::End => ChatKey::End,
        KeyCode::Backspace => ChatKey::Backspace,
        KeyCode::Delete => ChatKey::Delete,
        KeyCode::Esc => ChatKey::Esc,
        KeyCode::Char(c) => ChatKey::Char(c),
        _ => return None,
    })
}
