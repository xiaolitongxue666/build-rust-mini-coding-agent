//! 第 08 课：[Better input](https://www.byoharness.dev/chapters/08-better-input.html)
//!
//! 课上两步：readline → Bubble Tea 一次性输入框。本仓库步骤 1 是第 04 课的 rustyline；
//! 本课换成 crossterm 一次性 MVU（不上第 12 课整屏 TUI）。
//!
//! `buffer_text`：开始按 ↑ 之前正在打的字。↓ 越过最新一条时要还回去。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const BOLD_CYAN: &str = "\x1b[1;36m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const HISTORY_NAME: &str = ".rustbyo_harness_history";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKey {
    Enter,
    CtrlD,
    CtrlC,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
    Esc,
    Char(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatOutcome {
    Continue,
    Cleared,
    Submit(String),
    Quit,
}

#[derive(Debug, Clone)]
pub struct ChatInputState {
    value: String,
    cursor: usize,
    history: Vec<String>,
    /// -1 = 没在翻历史。
    hist_idx: i32,
    buffer_text: String,
    width: usize,
}

impl ChatInputState {
    pub fn new(width: usize, history: Vec<String>) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            history,
            hist_idx: -1,
            buffer_text: String::new(),
            width: width.max(24),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn apply(&mut self, key: ChatKey) -> ChatOutcome {
        match key {
            ChatKey::Enter => ChatOutcome::Submit(self.value.clone()),
            ChatKey::CtrlD => {
                if self.value.is_empty() {
                    ChatOutcome::Quit
                } else {
                    ChatOutcome::Continue
                }
            }
            ChatKey::CtrlC => {
                self.value.clear();
                self.cursor = 0;
                self.hist_idx = -1;
                self.buffer_text.clear();
                ChatOutcome::Cleared
            }
            ChatKey::Up => {
                self.history_up();
                ChatOutcome::Continue
            }
            ChatKey::Down => {
                self.history_down();
                ChatOutcome::Continue
            }
            ChatKey::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                ChatOutcome::Continue
            }
            ChatKey::Right => {
                if self.cursor < self.chars().len() {
                    self.cursor += 1;
                }
                ChatOutcome::Continue
            }
            ChatKey::Home => {
                self.cursor = 0;
                ChatOutcome::Continue
            }
            ChatKey::End => {
                self.cursor = self.chars().len();
                ChatOutcome::Continue
            }
            ChatKey::Backspace => {
                if self.cursor > 0 {
                    let mut chars = self.chars();
                    chars.remove(self.cursor - 1);
                    self.cursor -= 1;
                    self.value = chars.into_iter().collect();
                }
                ChatOutcome::Continue
            }
            ChatKey::Delete => {
                let mut chars = self.chars();
                if self.cursor < chars.len() {
                    chars.remove(self.cursor);
                    self.value = chars.into_iter().collect();
                }
                ChatOutcome::Continue
            }
            ChatKey::Esc => ChatOutcome::Continue,
            ChatKey::Char(c) => {
                let mut chars = self.chars();
                let at = self.cursor.min(chars.len());
                chars.insert(at, c);
                self.cursor = at + 1;
                self.value = chars.into_iter().collect();
                ChatOutcome::Continue
            }
        }
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.hist_idx < 0 {
            self.buffer_text = self.value.clone();
            self.hist_idx = 0;
        } else if (self.hist_idx as usize) < self.history.len() - 1 {
            self.hist_idx += 1;
        }
        self.set_from_history();
    }

    fn history_down(&mut self) {
        if self.hist_idx < 0 {
            return;
        }
        if self.hist_idx == 0 {
            self.hist_idx = -1;
            self.value = self.buffer_text.clone();
            self.cursor = self.chars().len();
            return;
        }
        self.hist_idx -= 1;
        self.set_from_history();
    }

    fn set_from_history(&mut self) {
        let idx = self.history.len() - 1 - self.hist_idx as usize;
        self.value = self.history[idx].clone();
        self.cursor = self.chars().len();
    }

    fn chars(&self) -> Vec<char> {
        self.value.chars().collect()
    }
}

/// 第 08 课 View：圆角框 + 底下一行 hint。不进 alt-screen，提交后框留在屏幕上。
pub fn render_box(state: &ChatInputState) -> String {
    let inner = (state.width.saturating_sub(2)).max(20);
    let prompt = format!("❯ {}", state.value);
    let mut body = prompt;
    let body_len = body.chars().count();
    if body_len < inner {
        body.push_str(&" ".repeat(inner - body_len));
    } else {
        body = body.chars().take(inner).collect();
    }
    let bar = "─".repeat(inner);
    format!(
        "{BOLD_CYAN}╭{bar}╮{RESET}\n{BOLD_CYAN}│{RESET}{body}{BOLD_CYAN}│{RESET}\n{BOLD_CYAN}╰{bar}╯{RESET}\n{DIM} enter: send · ↑↓: history · ctrl-d: exit{RESET}"
    )
}

/// 只认当前环境 `$HOME`（空则 Windows 的 `USERPROFILE`）。不跨 OS 写。
pub fn history_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home).join(HISTORY_NAME));
        }
    }
    std::env::var("USERPROFILE")
        .ok()
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(HISTORY_NAME))
}

pub fn load_history() -> Vec<String> {
    history_path()
        .map(|path| load_history_from(&path))
        .unwrap_or_default()
}

pub fn load_history_from(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn append_history(line: &str) {
    if let Some(path) = history_path() {
        append_history_to(&path, line);
    }
}

pub fn append_history_to(path: &Path, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_saves_buffer_down_restores() {
        let mut state = ChatInputState::new(40, vec!["old".into(), "new".into()]);
        state.apply(ChatKey::Char('x'));
        state.apply(ChatKey::Up);
        assert_eq!(state.value(), "new");
        state.apply(ChatKey::Up);
        assert_eq!(state.value(), "old");
        state.apply(ChatKey::Down);
        assert_eq!(state.value(), "new");
        state.apply(ChatKey::Down);
        assert_eq!(state.value(), "x");
    }

    #[test]
    fn render_box_has_border_and_hint() {
        let state = ChatInputState::new(40, Vec::new());
        let view = render_box(&state);
        assert!(view.contains('╭'), "{view}");
        assert!(view.contains('❯'), "{view}");
        assert!(view.contains("enter: send"), "{view}");
    }

    #[test]
    fn history_file_roundtrip() {
        use std::fs;
        let path = std::env::temp_dir().join(format!(
            "byo-hist-{}.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        append_history_to(&path, "one");
        append_history_to(&path, "");
        append_history_to(&path, "two");
        let lines = load_history_from(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
    }
}
