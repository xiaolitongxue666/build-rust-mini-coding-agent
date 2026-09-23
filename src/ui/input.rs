//! 第 04 课指定键位：↑↓ 会话历史、Ctrl+C 一次清空、两次静默退出。
//! 任务中 Esc 在 spinner 里处理。confirm 必须共用这把 editor，不能再 lock stdin。
//! 不写 HistoryFile（第 08 课陷阱：历史文件会留下贴进去的密钥）。

use std::io::{self, Write};

use rustyline::error::ReadlineError;
use rustyline::{Cmd, DefaultEditor, EventHandler, KeyCode, KeyEvent, Modifiers};

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
    editor: DefaultEditor,
    ctrl_c_streak: u8,
}

impl SessionInput {
    pub fn new() -> Result<Self, String> {
        let editor = DefaultEditor::new().map_err(|e| e.to_string())?;
        // Esc 只在 confirm 时绑 Abort；空闲提示行保持 rustyline 默认（不当退出）。
        Ok(Self {
            editor,
            ctrl_c_streak: 0,
        })
    }

    pub fn read_repl(&mut self) -> ReplLine {
        self.unbind_esc();
        loop {
            match self.editor.readline("> ") {
                Ok(line) => {
                    self.ctrl_c_streak = 0;
                    let input = line.trim();
                    if input.is_empty() {
                        continue;
                    }
                    let _ = self.editor.add_history_entry(input);
                    return ReplLine::Text(input.to_string());
                }
                Err(ReadlineError::Interrupted) => {
                    self.ctrl_c_streak = self.ctrl_c_streak.saturating_add(1);
                    if ctrl_c_action(self.ctrl_c_streak) == CtrlCAction::Quit {
                        return ReplLine::Quit;
                    }
                }
                Err(ReadlineError::Eof) | Err(_) => return ReplLine::Quit,
            }
        }
    }

    fn bind_esc_abort(&mut self) {
        self.editor.bind_sequence(
            KeyEvent(KeyCode::Esc, Modifiers::NONE),
            EventHandler::Simple(Cmd::Interrupt),
        );
    }

    fn unbind_esc(&mut self) {
        let _ = self
            .editor
            .unbind_sequence(KeyEvent(KeyCode::Esc, Modifiers::NONE));
    }
}

impl PromptRead for SessionInput {
    fn read_line(&mut self, prompt: &str) -> LineResult {
        self.bind_esc_abort();
        let result = match self.editor.readline(prompt) {
            Ok(line) => {
                self.ctrl_c_streak = 0;
                LineResult::Line(line)
            }
            Err(ReadlineError::Interrupted) => LineResult::Abort,
            Err(ReadlineError::Eof) | Err(_) => LineResult::End,
        };
        self.unbind_esc();
        result
    }
}
