//! 第 04 课：API 等待时的 braille spinner。
//! `\r` 回到列 0，`\033[K` 清到行尾。`Stop` 必须等线程确认清行，否则下一帧会盖住模型输出。
//! 非 TTY 是空壳，避免把 `\r⠋` 打进日志。

use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind};

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const TICK: Duration = Duration::from_millis(80);
const BOLD_CYAN: &str = "\x1b[1;36m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

pub fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

fn want_spinner() -> bool {
    // cargo test 会设 RUST_TEST_THREADS；此时 stdout 仍可能是 TTY，不要 raw mode / spinner。
    stdout_is_tty() && std::env::var_os("RUST_TEST_THREADS").is_none()
}

pub struct Spinner {
    stop: Option<Arc<Mutex<bool>>>,
    done: Option<JoinHandle<()>>,
}

impl Spinner {
    pub fn start(label: &str) -> Self {
        if !want_spinner() {
            return Self {
                stop: None,
                done: None,
            };
        }
        let stop = Arc::new(Mutex::new(false));
        let flag = Arc::clone(&stop);
        let label = label.to_string();
        let done = thread::spawn(move || {
            let mut i = 0usize;
            loop {
                if *flag.lock().unwrap_or_else(|e| e.into_inner()) {
                    let _ = write!(io::stdout(), "\r\x1b[K");
                    let _ = io::stdout().flush();
                    return;
                }
                let frame = FRAMES[i % FRAMES.len()];
                let _ = write!(
                    io::stdout(),
                    "\r{BOLD_CYAN}{frame}{RESET} {DIM}{label}{RESET}"
                );
                let _ = io::stdout().flush();
                i += 1;
                thread::sleep(TICK);
            }
        });
        Self {
            stop: Some(stop),
            done: Some(done),
        }
    }

    /// 第 04 课：必须等到 goroutine/线程确认清行。立刻返回会让下一帧打在模型文本上。
    pub fn stop(mut self) {
        if let Some(flag) = self.stop.take() {
            if let Ok(mut g) = flag.lock() {
                *g = true;
            }
        }
        if let Some(done) = self.done.take() {
            let _ = done.join();
        }
    }
}

pub enum Wait<T> {
    Done(T),
    Cancelled,
}

/// 有 TTY 时在旁路转 spinner，主线程看 Esc。取消后丢弃迟到的 `send` 结果。
pub fn spin_until<T: Send + 'static>(
    label: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> Wait<T> {
    if !want_spinner() {
        return Wait::Done(work());
    }

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(work());
    });

    let spinner = Spinner::start(label);
    let raw = crossterm::terminal::enable_raw_mode().is_ok();
    let outcome = loop {
        match rx.recv_timeout(TICK) {
            Ok(value) => break Wait::Done(value),
            Err(RecvTimeoutError::Timeout) => {
                if poll_esc() {
                    break Wait::Cancelled;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break Wait::Cancelled,
        }
    };
    if raw {
        let _ = crossterm::terminal::disable_raw_mode();
    }
    drain_keys();
    spinner.stop();
    outcome
}

fn poll_esc() -> bool {
    while crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
        match crossterm::event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc => {
                return true;
            }
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

fn drain_keys() {
    while crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
}
